use anyhow::Result;
use content_inspector::{inspect, ContentType};
use rayon::prelude::*;
use std::{
    collections::HashMap,
    fs::File,
    io::{self, Read},
    path::Path,
};

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
    if config.json {
        // JSON array of objects
        Ok(serde_json::to_string_pretty(
            &files
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
        Ok(files
            .iter()
            .map(|f| {
                config
                    .output_template
                    .replace("FILE_PATH", &f.rel_path)
                    .replace("FILE_CONTENT", &f.content)
                    .replace("\\\\n", "\n") // replace literal "\\\\n" with newline
            })
            .collect::<Vec<_>>()
            .join("\n"))
    }
}
