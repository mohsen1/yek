use anyhow::Result;
use anyhow::anyhow;
use content_inspector::{inspect, ContentType};
use rayon::prelude::*;
use std::{
    collections::HashMap,
    fs::File,
    io::{self, Read},
    path::Path,
    str::FromStr,
};
use bytesize::ByteSize;

pub mod config;
pub mod defaults;
pub mod parallel;
pub mod priority;

use config::YekConfig;
use parallel::{process_files_parallel, ProcessedFile};
use priority::compute_recentness_boost;

/// Check if a file is likely text or binary by reading only a small chunk.
/// This avoids reading large files fully just to detect their type.
pub fn is_text_file(path: &Path, user_binary_extensions: &[String]) -> io::Result<bool> {
    // If extension is known to be binary, skip quickly
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        if user_binary_extensions.iter().any(|bin_ext| bin_ext == ext) {
            return Ok(false);
        }
    }

    // Short partial read to check if it's binary or text
    const INSPECTION_BYTES: usize = 8192;
    let mut file = File::open(path)?;
    let mut buf = vec![0u8; INSPECTION_BYTES];
    let n = file.read(&mut buf)?;
    buf.truncate(n);

    Ok(inspect(&buf) != ContentType::BINARY)
}

/// Main entrypoint for serialization, used by CLI and tests
pub fn serialize_repo(config: &YekConfig) -> Result<(String, Vec<ProcessedFile>)> {
    // Gather commit times from each input dir
    let combined_commit_times = config
        .input_dirs
        .par_iter()
        .filter_map(|dir| {
            let repo_path = Path::new(dir);
            priority::get_recent_commit_times_git2(
                repo_path,
                config.max_git_depth.try_into().unwrap_or(0),
            )
        })
        .flatten()
        .collect::<HashMap<String, u64>>();

    // Compute a recentness-based boost
    let recentness_boost =
        compute_recentness_boost(&combined_commit_times, config.git_boost_max.unwrap_or(100));

    // Process files in parallel for each directory
    let merged_files = config
        .input_dirs
        .par_iter()
        .map(|dir| {
            let path = Path::new(dir);
            process_files_parallel(path, config, &recentness_boost)
        })
        .collect::<Result<Vec<Vec<ProcessedFile>>>>()?
        .into_iter()
        .flatten()
        .collect::<Vec<ProcessedFile>>();

    let mut files = merged_files;

    // Sort final (priority desc, then file_index asc)
    files.par_sort_by(|a, b| {
        a.priority
            .cmp(&b.priority)
            .reverse()
            .then_with(|| a.file_index.cmp(&b.file_index))
    });

    // Build the final output string
    let output_string = concat_files(&files, config)?;

    Ok((output_string, files))
}

pub fn concat_files(files: &[ProcessedFile], config: &YekConfig) -> anyhow::Result<String> {
    let mut accumulated = 0_usize;
    let cap = if config.token_mode {
        parse_token_limit(&config.tokens)?
    } else {
        ByteSize::from_str(&config.max_size)
            .map_err(|e| anyhow!("max_size: Invalid size format: {}", e))?
            .as_u64() as usize
    };

    let mut files_to_include = Vec::new();
    for file in files {
        let content_size = if config.token_mode {
            // Format the file content with template first, then count tokens
            let formatted = if config.json {
                serde_json::to_string(&serde_json::json!({
                    "filename": &file.rel_path,
                    "content": &file.content,
                })).unwrap_or_default()
            } else {
                config
                    .output_template
                    .replace("FILE_PATH", &file.rel_path)
                    .replace("FILE_CONTENT", &file.content)
            };
            count_tokens(&formatted)
        } else {
            file.content.len()
        };

        if accumulated + content_size <= cap {
            accumulated += content_size;
            files_to_include.push(file);
        } else {
            break;
        }
    }

    if config.json {
        // JSON array of objects
        Ok(serde_json::to_string_pretty(
            &files_to_include
                .iter()
                .map(|f| {
                    serde_json::json!({
                        "filename": &f.rel_path,
                        "content": &f.content,
                    })
                })
                .collect::<Vec<_>>(),
        )?)
    } else {
        // Use the user-defined template
        Ok(files_to_include
            .iter()
            .map(|f| {
                config
                    .output_template
                    .replace("FILE_PATH", &f.rel_path)
                    .replace("FILE_CONTENT", &f.content)
                    // Handle both literal "\n" and escaped "\\n"
                    .replace("\\\\\n", "\n") // First handle escaped newline
                    .replace("\\\\n", "\n") // Then handle escaped \n sequence
            })
            .collect::<Vec<_>>()
            .join("\n"))
    }
}

/// Parse a token limit string like "800k" or "1000" into a number
fn parse_token_limit(limit: &str) -> anyhow::Result<usize> {
    if limit.to_lowercase().ends_with('k') {
        limit[..limit.len() - 1]
            .trim()
            .parse::<usize>()
            .map(|n| n * 1000)
            .map_err(|e| anyhow!("tokens: Invalid token size: {}", e))
    } else {
        limit
            .parse::<usize>()
            .map_err(|e| anyhow!("tokens: Invalid token size: {}", e))
    }
}

/// Count tokens in a string by splitting on whitespace and punctuation
fn count_tokens(text: &str) -> usize {
    text.split(|c: char| c.is_whitespace() || c.is_ascii_punctuation())
        .filter(|s| !s.is_empty())
        .count()
}
