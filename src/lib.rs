use anyhow::Result;
use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::io::{self, Write};
use std::path::Path;

pub mod config;
mod defaults;
mod parallel;
mod priority;

use config::FullYekConfig;
use defaults::{BINARY_FILE_EXTENSIONS, TEXT_FILE_EXTENSIONS};
use parallel::{process_files_parallel, ProcessedFile};

/// The main function that the tests call.
pub fn serialize_repo(config: &FullYekConfig) -> Result<()> {
    let mut processed_files = Vec::<ProcessedFile>::new();

    // TODO: Small perf low hanging fruit: process all dirs in parallel
    for dir in &config.input_dirs {
        let path = Path::new(dir);
        // Process files in parallel
        let dir_files = process_files_parallel(path, config)?;
        processed_files.extend(dir_files);
    }
    // Convert to the format expected by write_chunks
    let entries: Vec<(String, String, i32)> = processed_files
        .into_iter()
        .map(|f| (f.rel_path, f.content, f.priority))
        .collect();

    let output_string = entries
        .iter()
        .map(|e| e.1.to_string())
        .collect::<Vec<String>>()
        .join("\n");

    write_output(&output_string, config)?;

    Ok(())
}

/// Rank-based approach to compute how "recent" each file is (0=oldest, 1=newest).
/// Then scale it to a user-defined or default max boost.
#[allow(dead_code)]
fn compute_recentness_boost(
    commit_times: &HashMap<String, u64>,
    max_boost: i32,
) -> HashMap<String, i32> {
    if commit_times.is_empty() {
        return HashMap::new();
    }

    // Sort by ascending commit time => first is oldest
    let mut sorted: Vec<(&String, &u64)> = commit_times.iter().collect();
    sorted.sort_by_key(|(_, t)| **t);

    // oldest file => rank=0, newest => rank=1
    let last_index = sorted.len().saturating_sub(1) as f64;
    if last_index < 1.0 {
        // If there's only one file, or zero, no boosts make sense
        let mut single = HashMap::new();
        for file in commit_times.keys() {
            single.insert(file.clone(), 0);
        }
        return single;
    }

    let mut result = HashMap::new();
    for (i, (path, _time)) in sorted.iter().enumerate() {
        let rank = i as f64 / last_index; // 0.0..1.0 (older files get lower rank)
        let boost = (rank * max_boost as f64).round() as i32; // Newer files get higher boost
        result.insert((*path).clone(), boost);
    }
    result
}

#[cfg(target_family = "windows")]
#[allow(dead_code)]
fn is_effectively_absolute(path: &std::path::Path) -> bool {
    if path.is_absolute() {
        return true;
    }
    // Also treat a leading slash/backslash as absolute
    match path.to_str() {
        Some(s) => s.starts_with('/') || s.starts_with('\\'),
        None => false,
    }
}

#[cfg(not(target_family = "windows"))]
#[allow(dead_code)]
fn is_effectively_absolute(path: &std::path::Path) -> bool {
    path.is_absolute()
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
