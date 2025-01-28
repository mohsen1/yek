use anyhow::Result;
use content_inspector::{inspect, ContentType};
use rayon::prelude::*;
use std::collections::HashMap;
use std::fs;
use std::io::{self, Write};
use std::iter::Iterator;
use std::path::Path;

pub mod config;
mod parallel;
pub mod priority;

use config::FullYekConfig;
use parallel::{process_files_parallel, ProcessedFile};
use priority::compute_recentness_boost;

/// Check if a file is a text file by examining its content
pub fn is_text_file(path: &Path, user_binary_extensions: &[String]) -> io::Result<bool> {
    // Check if the file has a binary extension
    if let Some(ext) = path.extension() {
        if let Some(ext_str) = ext.to_str() {
            if user_binary_extensions.iter().any(|e| e == ext_str) {
                return Ok(false);
            }
        }
    }

    // Read and inspect the file content
    let content = fs::read(path)?;
    Ok(inspect(&content) != ContentType::BINARY)
}

/// The main function that the tests call.
pub fn serialize_repo(config: &FullYekConfig) -> Result<(String, Vec<ProcessedFile>)> {
    // Gather commit times from each input directory
    let combined_commit_times = config
        .input_dirs
        .par_iter()
        .filter_map(|dir| {
            let repo_path = Path::new(dir);
            priority::get_recent_commit_times_git2(repo_path)
        })
        .flatten()
        .collect::<HashMap<String, u64>>();

    // Compute a recentness boost for each file
    let recentness_boost = compute_recentness_boost(&combined_commit_times, config.git_boost_max);

    let output_string = config
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

    let mut files = output_string;
    files.par_sort_by(|a, b| {
        a.priority
            .cmp(&b.priority)
            .reverse()
            .then_with(|| a.file_index.cmp(&b.file_index))
    });

    let output_string = files
        .clone()
        .into_iter()
        .map(|f| f.content)
        .collect::<Vec<_>>()
        .join("\n");

    write_output(&output_string, config)?;

    Ok((output_string, files))
}

/// Write a single chunk either to stdout or file
fn write_output(content: &str, config: &FullYekConfig) -> io::Result<()> {
    if config.stream {
        let mut stdout = io::stdout();
        write!(stdout, "{}", content)?;
        stdout.flush()?;
    } else {
        let path = Path::new(&config.output_file_full_path);
        fs::create_dir_all(path.parent().unwrap())?;
        fs::write(path, content.as_bytes())?;
    }
    Ok(())
}
