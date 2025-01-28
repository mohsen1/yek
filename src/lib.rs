use anyhow::Result;
use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::io::{self, Write};
use std::path::Path;

pub mod config;
mod defaults;
mod parallel;
pub mod priority;

use config::FullYekConfig;
use defaults::{BINARY_FILE_EXTENSIONS, TEXT_FILE_EXTENSIONS};
use parallel::{process_files_parallel, ProcessedFile};
use priority::{compute_recentness_boost, get_recent_commit_times};

pub use parallel::normalize_path;

/// The main function that the tests call.
pub fn serialize_repo(config: &FullYekConfig) -> Result<()> {
    // Gather commit times from each input directory
    let mut combined_commit_times = HashMap::new();
    for dir in &config.input_dirs {
        let repo_path = Path::new(dir);
        if let Some(ct) = get_recent_commit_times(repo_path) {
            for (file, ts) in ct {
                // If a file appears in multiple dirs, keep the latest commit time
                combined_commit_times
                    .entry(file)
                    .and_modify(|t| {
                        if ts > *t {
                            *t = ts;
                        }
                    })
                    .or_insert(ts);
            }
        }
    }

    // Compute a recentness boost for each file
    let recentness_boost = compute_recentness_boost(&combined_commit_times, config.git_boost_max);

    let mut processed_files = Vec::<ProcessedFile>::new();
    for dir in &config.input_dirs {
        let path = Path::new(dir);
        // Process files in parallel
        let dir_files = process_files_parallel(path, config, &recentness_boost)?;
        processed_files.extend(dir_files);
    }

    let output_string = processed_files
        .iter()
        .map(|f| f.content.clone())
        .collect::<Vec<_>>()
        .join("\n");

    write_output(&output_string, config)?;

    Ok(())
}

/// Write a single chunk either to stdout or file
fn write_output(content: &str, config: &FullYekConfig) -> io::Result<()> {
    if config.stream {
        let mut stdout = io::stdout();
        write!(stdout, "{}", content)?;
        stdout.flush()?;
    } else {
        let output_file_path = format!("{}.txt", config.output_dir);
        let path = Path::new(&output_file_path);
        fs::create_dir_all(path.parent().unwrap())?;
        fs::write(path, content.as_bytes())?;
    }
    Ok(())
}

/// Check if file is text by extension or scanning first chunk for null bytes.
pub fn is_text_file(path: &Path, user_binary_extensions: &[String]) -> io::Result<bool> {
    // First check extension - fast path
    if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
        let ext_lc = ext.to_lowercase();
        // If it's in the known text extensions list, it's definitely text
        if TEXT_FILE_EXTENSIONS.contains(&ext_lc.as_str()) {
            return Ok(true);
        }
        // If it's in the binary extensions list (built-in or user-defined), it's definitely binary
        if BINARY_FILE_EXTENSIONS.contains(&ext_lc.as_str())
            || user_binary_extensions
                .iter()
                .any(|e| e.trim_start_matches('.') == ext_lc)
        {
            return Ok(false);
        }
        // Unknown extension - treat as binary
        return Ok(false);
    }

    // No extension - scan content
    let mut file = fs::File::open(path)?;
    let mut buffer = [0; 512];
    let n = file.read(&mut buffer)?;

    // Check for null bytes which typically indicate binary content
    Ok(!buffer[..n].contains(&0))
}
