use anyhow::anyhow;
use anyhow::Result;
use bytesize::ByteSize;
use content_inspector::{inspect, ContentType};
use rayon::prelude::*;
use std::{
    collections::HashMap,
    fs::File,
    io::{self, Read},
    path::Path,
    str::FromStr,
    sync::OnceLock,
};
use tiktoken_rs::CoreBPE;

pub mod config;
pub mod defaults;
pub mod error;
pub mod main_new;
pub mod models;
pub mod parallel;
pub mod parallel_fixed;
pub mod pipeline;
pub mod priority;
pub mod repository;
pub mod tree;

use config::YekConfig;
use parallel::{process_files_parallel, ProcessedFile};
use priority::compute_recentness_boost;
use tree::generate_tree;

// Add a static BPE encoder for reuse
static TOKENIZER: OnceLock<CoreBPE> = OnceLock::new();

fn get_tokenizer() -> &'static CoreBPE {
    TOKENIZER.get_or_init(|| {
        tiktoken_rs::get_bpe_from_model("gpt-3.5-turbo").expect("Failed to load tokenizer")
    })
}

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
    // Validate input paths and warn about non-existent ones
    let mut non_existent_paths = Vec::new();

    for path_str in &config.input_paths {
        let path = Path::new(path_str);
        // Check if path exists as a file, directory, or could be a glob pattern
        if !path.exists() && !path_str.contains('*') && !path_str.contains('?') {
            non_existent_paths.push(path_str.clone());
        }
    }

    // If we have non-existent paths, warn the user
    if !non_existent_paths.is_empty() {
        for path in &non_existent_paths {
            eprintln!("Warning: Path '{}' does not exist", path);
        }
    }

    // Gather commit times from each input path that is a directory
    let combined_commit_times = config
        .input_paths
        .par_iter()
        .filter_map(|path_str| {
            let repo_path = Path::new(path_str);
            if repo_path.is_dir() {
                priority::get_recent_commit_times_git2(
                    repo_path,
                    config.max_git_depth.try_into().unwrap_or(0),
                )
            } else {
                None
            }
        })
        .flatten()
        .collect::<HashMap<String, u64>>();

    // Compute a recentness-based boost
    let recentness_boost =
        compute_recentness_boost(&combined_commit_times, config.git_boost_max.unwrap_or(100));

    // Process files in parallel for each input path
    let merged_files = config
        .input_paths
        .par_iter()
        .map(|path_str| {
            let path = Path::new(path_str);
            process_files_parallel(path, config, &recentness_boost)
        })
        .collect::<Result<Vec<Vec<ProcessedFile>>>>()?
        .into_iter()
        .flatten()
        .collect::<Vec<ProcessedFile>>();

    let mut files = merged_files;

    // Sort final (priority asc, then file_index asc)
    files.par_sort_by(|a, b| {
        a.priority
            .cmp(&b.priority)
            .then_with(|| a.rel_path.cmp(&b.rel_path))
    });

    // If no files were processed and we had non-existent paths, provide additional context
    if files.is_empty() && !non_existent_paths.is_empty() {
        eprintln!("Warning: No files were processed. All specified paths were non-existent or contained no valid files.");
    }

    // Build the final output string
    let output_string = concat_files(&files, config)?;

    // Only count tokens if debug logging is enabled
    if tracing::Level::DEBUG <= tracing::level_filters::STATIC_MAX_LEVEL {
        tracing::debug!("{} tokens generated", count_tokens(&output_string));
    }

    Ok((output_string, files))
}

pub fn concat_files(files: &[ProcessedFile], config: &YekConfig) -> anyhow::Result<String> {
    // Generate tree header if requested
    let tree_header = if config.tree_header || config.tree_only {
        let file_paths: Vec<std::path::PathBuf> = files
            .iter()
            .map(|f| std::path::PathBuf::from(&f.rel_path))
            .collect();
        generate_tree(&file_paths)
    } else {
        String::new()
    };

    // If tree_only is requested, return just the tree
    if config.tree_only {
        return Ok(tree_header);
    }

    let mut accumulated = 0_usize;
    let cap = if config.token_mode {
        parse_token_limit(&config.tokens)?
    } else {
        ByteSize::from_str(&config.max_size)
            .map_err(|e| anyhow!("max_size: Invalid size format: {}", e))?
            .as_u64() as usize
    };

    // Account for tree header size in capacity calculations
    let tree_header_size = if config.tree_header {
        if config.token_mode {
            count_tokens(&tree_header)
        } else {
            tree_header.len()
        }
    } else {
        0
    };

    accumulated += tree_header_size;

    // Sort by priority (asc) and file_index (asc)
    let mut sorted_files: Vec<_> = files.iter().collect();
    sorted_files.sort_by(|a, b| {
        a.priority
            .cmp(&b.priority)
            .then_with(|| a.rel_path.cmp(&b.rel_path))
    });

    let mut files_to_include = Vec::new();
    for file in sorted_files {
        let content_size = if config.token_mode {
            // Format the file content with template first, then count tokens
            let content = format_content_with_line_numbers(&file.content, config.line_numbers);
            let formatted = if config.json {
                serde_json::to_string(&serde_json::json!({
                    "filename": &file.rel_path,
                    "content": content,
                }))
                .map_err(|e| anyhow!("Failed to serialize JSON: {}", e))?
            } else {
                config
                    .output_template
                    .as_ref()
                    .expect("output_template should be set")
                    .replace("FILE_PATH", &file.rel_path)
                    .replace("FILE_CONTENT", &content)
                    // Handle both literal "\n" and escaped "\\n"
                    .replace("\\\\\n", "\n") // First handle escaped newline
                    .replace("\\\\n", "\n") // Then handle escaped \n sequence
            };
            count_tokens(&formatted)
        } else {
            let content = format_content_with_line_numbers(&file.content, config.line_numbers);
            content.len()
        };

        if accumulated + content_size <= cap {
            accumulated += content_size;
            files_to_include.push(file);
        } else {
            break;
        }
    }

    let main_content = if config.json {
        // JSON array of objects
        serde_json::to_string_pretty(
            &files_to_include
                .iter()
                .map(|f| {
                    let content = format_content_with_line_numbers(&f.content, config.line_numbers);
                    serde_json::json!({
                        "filename": &f.rel_path,
                        "content": content,
                    })
                })
                .collect::<Vec<_>>(),
        )?
    } else {
        // Use the user-defined template
        files_to_include
            .iter()
            .map(|f| {
                let content = format_content_with_line_numbers(&f.content, config.line_numbers);
                config
                    .output_template
                    .as_ref()
                    .expect("output_template should be set")
                    .replace("FILE_PATH", &f.rel_path)
                    .replace("FILE_CONTENT", &content)
                    // Handle both literal "\n" and escaped "\\n"
                    .replace("\\\\\n", "\n") // First handle escaped newline
                    .replace("\\\\n", "\n") // Then handle escaped \n sequence
            })
            .collect::<Vec<_>>()
            .join("\n")
    };

    // Combine tree header with main content
    if config.tree_header {
        Ok(format!("{}{}", tree_header, main_content))
    } else {
        Ok(main_content)
    }
}

/// Format file content with line numbers if requested
fn format_content_with_line_numbers(content: &str, include_line_numbers: bool) -> String {
    if !include_line_numbers {
        return content.to_string();
    }

    let lines: Vec<&str> = content.lines().collect();
    let total_lines = lines.len();

    // Calculate the width needed for the largest line number, with minimum width of 3
    let width = if total_lines == 0 {
        3
    } else {
        std::cmp::max(3, total_lines.to_string().len())
    };

    lines
        .iter()
        .enumerate()
        .map(|(i, line)| format!("{:width$} | {}", i + 1, line, width = width))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Parse a token limit string like "800k" or "1000" into a number
pub fn parse_token_limit(limit: &str) -> anyhow::Result<usize> {
    if limit.to_lowercase().ends_with('k') {
        // Use UTF-8 aware slicing to handle emojis and other multi-byte characters
        let chars: Vec<char> = limit.chars().collect();
        if chars.len() > 1 {
            chars[..chars.len() - 1]
                .iter()
                .collect::<String>()
                .trim()
                .parse::<usize>()
                .map(|n| n * 1000)
                .map_err(|e| anyhow!("tokens: Invalid token size: {}", e))
        } else {
            Err(anyhow!("tokens: Invalid token format: {}", limit))
        }
    } else {
        limit
            .parse::<usize>()
            .map_err(|e| anyhow!("tokens: Invalid token size: {}", e))
    }
}

/// Count tokens using tiktoken's GPT-3.5-Turbo tokenizer for accuracy
pub fn count_tokens(text: &str) -> usize {
    get_tokenizer().encode_with_special_tokens(text).len()
}
