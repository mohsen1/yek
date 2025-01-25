use crate::{
    get_file_priority, get_recent_commit_times, is_ignored, is_text_file,
    model_manager::{self},
    normalize_path_with_root, Result, YekConfig,
};
use anyhow::anyhow;
use ignore::{DirEntry, WalkBuilder, WalkState};
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
            let rel_path = normalize_path_with_root(&base_dir, path);

            // Skip if path is ignored
            if is_ignored(&rel_path, &config.ignore_patterns) {
                debug!("Skipping ignored file: {}", rel_path);
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

            // Read file content
            let content = match std::fs::read_to_string(path) {
                Ok(content) => content,
                Err(e) => {
                    debug!("Error reading file {}: {}", rel_path, e);
                    return WalkState::Continue;
                }
            };

            // Calculate priority
            let _priority = get_file_priority(&rel_path, &config.priority_rules);

            let model = config.tokenizer_model.as_deref().unwrap_or("openai");
            // TODO: this is not really working right now
            let (entry_header, _header_size, _content_size) = get_entry_header_with_size(
                &entry,
                0,
                0
            );

            // Calculate total entry size including header and content
            let content_with_newline = format!("{}\n", content);
            if config.token_mode {
                // TOKEN-MODE truncation logic
                let header_tokens = match model_manager::tokenize(&entry_header, model) {
                    Ok(tokens) => tokens,
                    Err(e) => {
                        debug!("Error tokenizing header: {}", e);
                        return WalkState::Continue;
                    }
                };
                let content_tokens = match model_manager::tokenize(&content_with_newline, model) {
                    Ok(tokens) => tokens,
                    Err(e) => {
                        debug!("Error tokenizing content: {}", e);
                        return WalkState::Continue;
                    }
                };
                let total_tokens_needed = header_tokens.len() + content_tokens.len();
                debug!(
                    "Processing file {} in token mode - header tokens: {}, content tokens: {}, total needed: {}, current: {}, max: {}",
                    rel_path,
                    header_tokens.len(),
                    content_tokens.len(),
                    total_tokens_needed,
                    0,
                    0
                );

                if total_tokens_needed > 0 {
                    return WalkState::Continue;
                }

                // Only check max size if it's set
                if let Some(max_size) = config.max_size {
                    if total_tokens_needed > max_size {
                        return WalkState::Continue;
                    }
                }

                if let Ok(mut output) = shared_output.lock() {
                    output.push_str(&entry_header);
                    output.push_str(&content_with_newline);
                }
            } else {
                // BYTE-MODE truncation logic
                let (entry_header, header_size, content_size) = get_entry_header_with_size(
                    &entry,
                    0,
                    0
                );

                // Only check max size if it's set
                if let Some(max_size) = config.max_size {
                    if header_size + content_size > max_size {
                        return WalkState::Continue;
                    }
                }

                if let Ok(mut output) = shared_output.lock() {
                    output.push_str(&entry_header);
                    output.push_str(&content_with_newline);
                }
            }

            WalkState::Continue
        })
    });

    // Copy shared output back to output_content
    if let Ok(shared) = shared_output.lock() {
        output_content.push_str(&shared);
    }

    Ok(())
}

fn get_entry_header_with_size(
    entry: &DirEntry,
    _current_bytes: usize,
    _max_bytes: usize,
) -> (String, usize, usize) {
    let rel_path = entry
        .path()
        .strip_prefix(entry.path().parent().unwrap())
        .unwrap();
    let header = format!(">>>> {}\n", rel_path.display());
    let header_len = header.len();
    (header, header_len, 0)
}
