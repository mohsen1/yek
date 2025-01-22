use crate::{
    get_file_priority, get_recent_commit_times, glob_to_regex,
    model_manager::{self},
    normalize_path, Result, YekConfig,
};
use anyhow::anyhow;
use crossbeam::channel::bounded;
use ignore::{WalkBuilder, WalkState};
use regex::Regex;
use std::{
    collections::HashSet,
    path::Path,
    sync::{Arc, Mutex},
};
use tracing::debug;

#[derive(Debug)]
pub struct ProcessedFile {
    pub priority: i32,
    pub rel_path: String,
    pub content: String,
}

pub fn process_files_parallel(
    base_dir: &Path,
    config: &YekConfig,
    chunks: &mut Vec<String>,
) -> Result<()> {
    // Validate token mode configuration first
    if config.token_mode {
        let model = config.tokenizer_model.as_deref().unwrap_or("openai");
        if !model_manager::SUPPORTED_MODEL_FAMILIES.contains(&model) {
            return Err(anyhow!(
                "Unsupported model '{}'. Supported models: {}",
                model,
                model_manager::SUPPORTED_MODEL_FAMILIES.join(", ")
            ));
        }
    }

    // Get Git commit times for prioritization
    let git_times = get_recent_commit_times(base_dir);
    debug!("Git commit times: {:?}", git_times);

    let (tx, rx) = bounded(1024);
    let _num_threads = num_cpus::get().min(16); // Cap at 16 threads

    let config = Arc::new(config.clone());
    let base_dir = Arc::new(base_dir.to_path_buf());
    let processed_files = Arc::new(Mutex::new(HashSet::<String>::new()));
    let file_counter = Arc::new(Mutex::new(0_usize));

    let walker = WalkBuilder::new(&*base_dir)
        .hidden(true)
        .ignore(true)
        .git_ignore(true)
        .build_parallel();

    walker.run(|| {
        let tx = tx.clone();
        let base_dir = Arc::clone(&base_dir);
        let config = Arc::clone(&config);
        let processed_files = Arc::clone(&processed_files);
        let file_counter = Arc::clone(&file_counter);

        Box::new(move |entry| {
            let entry = match entry {
                Ok(entry) => entry,
                Err(_) => return WalkState::Continue,
            };

            if !entry.file_type().map_or(false, |ft| ft.is_file()) {
                return WalkState::Continue;
            }

            let rel_path = normalize_path(entry.path(), &base_dir);
            let priority = get_file_priority(&rel_path, &config.priority_rules);

            // Check if it's a binary file
            let path = entry.path();
            let normalized_ext = path
                .extension()
                .and_then(|s| s.to_str())
                .map(|s| s.trim_start_matches('.').to_lowercase())
                .unwrap_or_default();

            let is_binary = config
                .binary_extensions
                .iter()
                .any(|ext| ext.trim_start_matches('.').to_lowercase() == normalized_ext);

            if is_binary {
                debug!("Skipping binary file: {}", rel_path);
                return WalkState::Continue;
            }

            // Read and process file
            if let Ok(content) = std::fs::read_to_string(entry.path()) {
                let processed = ProcessedFile {
                    priority,
                    rel_path,
                    content,
                };
                if tx.send(processed).is_err() {
                    return WalkState::Quit;
                }
            }

            WalkState::Continue
        })
    });

    drop(tx);

    let results: Vec<ProcessedFile> = rx.iter().collect();
    let mut sorted_results = results;
    sorted_results.sort_by(|a, b| b.priority.cmp(&a.priority));

    let max_size = config.max_size.unwrap_or(usize::MAX);
    let mut current_chunk = String::new();
    let mut current_size = 0;

    // Build chunks
    for entry in sorted_results {
        let model = config.tokenizer_model.as_deref().unwrap_or("openai");
        let entry_header = format!(">>>> {}\n", entry.rel_path);
        let entry_content = format!("{}{}\n", entry_header, entry.content);
        let entry_size = if config.token_mode {
            model_manager::count_tokens(&entry_content, model).unwrap_or_else(|e| {
                tracing::warn!("Token count failed for {}: {}", entry.rel_path, e);
                entry_content.len()
            })
        } else {
            entry_content.len()
        };

        if entry_size > max_size {
            // Handle large files by splitting
            let mut start = 0;
            let mut part = 0;
            while start < entry.content.len() {
                let part_header = format!(">>>> {} (part {})\n", entry.rel_path, part);
                let available_size = max_size.saturating_sub(part_header.len());
                let end = start + available_size.min(entry.content.len() - start);
                let chunk = format!("{}{}\n", part_header, &entry.content[start..end]);
                chunks.push(chunk);
                start = end;
                part += 1;
            }
        } else if current_size + entry_size > max_size {
            // Start new chunk
            if !current_chunk.is_empty() {
                chunks.push(current_chunk);
                current_chunk = String::new();
                current_size = 0;
            }
            current_chunk.push_str(&entry_content);
            current_size = entry_size;
        } else {
            current_chunk.push_str(&entry_content);
            current_size += entry_size;
        }
    }

    // Add final chunk if not empty
    if !current_chunk.is_empty() {
        chunks.push(current_chunk);
    }

    Ok(())
}
