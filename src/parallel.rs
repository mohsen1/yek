use crate::{
    get_recent_commit_times, is_ignored, is_text_file,
    model_manager::{self},
    normalize_path_with_root, Result, YekConfig,
};
use anyhow::anyhow;
use ignore::{WalkBuilder, WalkState};
use std::{
    path::Path,
    sync::{Arc, Mutex},
};
use tracing::debug;

pub fn process_files_parallel(
    base_dir: &Path,
    config: &YekConfig,
    output_content: &mut String,
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
        debug!("Token mode enabled with model: {}", model);
    }

    // Get Git commit times for prioritization
    let git_times = get_recent_commit_times(base_dir);
    debug!("Git commit times: {:?}", git_times);

    // Create thread-safe shared output content
    let shared_output = Arc::new(Mutex::new(String::new()));

    // Process files in parallel
    let walker = WalkBuilder::new(base_dir).build_parallel();
    walker.run(|| {
        let base_dir = base_dir.to_path_buf();
        let config = config.clone();
        let shared_output = Arc::clone(&shared_output);
        let git_times = git_times.clone();
        Box::new(move |entry| {
            let entry = match entry {
                Ok(entry) => entry,
                Err(e) => {
                    debug!("Error walking directory: {}", e);
                    return WalkState::Continue;
                }
            };

            if !entry.file_type().is_some_and(|ft| ft.is_file()) {
                return WalkState::Continue;
            }

            let path = entry.path();
            let rel_path = normalize_path_with_root(path, &base_dir);

            // Calculate priority based on git history
            let priority = if let Some(times) = &git_times {
                times.get(&rel_path).copied().unwrap_or(0)
            } else {
                0
            };

            // Skip if path is ignored
            if is_ignored(&rel_path, &config.ignore_patterns) {
                debug!("Skipping ignored path: {}", rel_path);
                return WalkState::Continue;
            }

            // Skip if not a text file
            if !is_text_file(path, &config.binary_extensions).unwrap_or_else(|e| {
                debug!("Error checking if file is text: {}", e);
                false
            }) {
                debug!("Skipping binary file: {}", rel_path);
                return WalkState::Continue;
            }

            // Process file based on priority
            let mut output = shared_output.lock().unwrap();
            let file_content = match process_file(&rel_path, &base_dir, &config) {
                Ok(content) => content,
                Err(e) => {
                    debug!("Error processing file {}: {}", rel_path, e);
                    return WalkState::Continue;
                }
            };

            // Insert content based on priority
            if priority > 0 {
                // Higher priority files go at the start
                output.insert_str(0, &file_content);
            } else {
                // Lower priority files go at the end
                output.push_str(&file_content);
            }

            WalkState::Continue
        })
    });

    // Copy shared output back to output_content
    if let Ok(shared) = shared_output.lock() {
        output_content.push_str(&shared);
    } else {
        return Err(anyhow!("Failed to acquire final lock for output"));
    }

    Ok(())
}

fn process_file(rel_path: &str, base_dir: &Path, config: &YekConfig) -> Result<String> {
    let path = base_dir.join(rel_path);
    let content = std::fs::read_to_string(&path)?;
    let model = config.tokenizer_model.as_deref().unwrap_or("openai");
    let entry_header = format!(">>>> {}\n", rel_path);
    let content_with_newline = format!("{}\n", content);

    // Check size limits before processing
    if let Some(max_size) = config.max_size {
        if config.token_mode {
            // TOKEN-MODE size check
            let header_tokens = model_manager::tokenize(&entry_header, model)?;
            let content_tokens = model_manager::tokenize(&content_with_newline, model)?;
            let total_tokens = header_tokens.len() + content_tokens.len();

            if total_tokens > max_size {
                debug!(
                    "File {} exceeds token limit: {} > {}",
                    rel_path, total_tokens, max_size
                );
                return Err(anyhow!("File too large"));
            }
        } else {
            // BYTE-MODE size check
            let total_bytes = entry_header.len() + content_with_newline.len();
            if total_bytes > max_size {
                debug!(
                    "File {} exceeds byte limit: {} > {}",
                    rel_path, total_bytes, max_size
                );
                return Err(anyhow!("File too large"));
            }
        }
    }

    Ok(format!("{}{}", entry_header, content_with_newline))
}
