use crate::{
    get_file_priority, get_recent_commit_times, is_ignored, is_text_file,
    model_manager::{self},
    normalize_path_with_root, Result, YekConfig,
};
use anyhow::anyhow;
use crossbeam::channel::bounded;
use ignore::{WalkBuilder, WalkState};
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
    output_chunks: &mut Vec<String>,
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

    // Process files in parallel
    let walker = WalkBuilder::new(&*base_dir)
        .hidden(false)
        .ignore(true)
        .git_ignore(true)
        .build_parallel();

    walker.run(|| {
        let tx = tx.clone();
        let config = Arc::clone(&config);
        let base_dir = Arc::clone(&base_dir);
        let processed_files = Arc::clone(&processed_files);
        let file_counter = Arc::clone(&file_counter);

        Box::new(move |entry| {
            let entry = match entry {
                Ok(entry) => entry,
                Err(err) => {
                    debug!("Error walking directory: {}", err);
                    return WalkState::Continue;
                }
            };

            if !entry.file_type().is_some_and(|ft| ft.is_file()) {
                return WalkState::Continue;
            }

            let path = entry.path();
            let rel_path = normalize_path_with_root(path, &base_dir);

            // Skip if already processed
            {
                let mut processed = processed_files.lock().unwrap();
                if processed.contains(&rel_path) {
                    return WalkState::Continue;
                }
                processed.insert(rel_path.clone());
            }

            // Skip git directory
            if rel_path.starts_with(".git/") {
                return WalkState::Continue;
            }

            // Check if file should be ignored
            if is_ignored(&rel_path, &config.ignore_patterns) {
                debug!("Ignoring file: {}", rel_path);
                return WalkState::Continue;
            }

            // Check if file is binary
            if let Ok(is_text) = is_text_file(path, &config.binary_extensions) {
                if !is_text {
                    debug!("Skipping binary file: {}", path.display());
                    return WalkState::Continue;
                }
            } else {
                debug!("Error checking if file is text: {}", path.display());
                return WalkState::Continue;
            }

            // Read file content
            let content = match std::fs::read_to_string(path) {
                Ok(content) => content,
                Err(err) => {
                    debug!("Error reading file {}: {}", path.display(), err);
                    return WalkState::Continue;
                }
            };

            // Get file priority
            let priority = get_file_priority(&rel_path, &config.priority_rules);

            // Send processed file
            if let Err(err) = tx.send(ProcessedFile {
                priority,
                rel_path,
                content,
            }) {
                debug!("Error sending file: {}", err);
            }

            // Increment file counter
            let mut counter = file_counter.lock().unwrap();
            *counter += 1;

            WalkState::Continue
        })
    });

    // Drop sender to signal completion
    drop(tx);

    // Collect and sort results
    let mut sorted_results: Vec<ProcessedFile> = rx.iter().collect();
    sorted_results.sort_by(|a, b| {
        // Sort by ascending priority (lower priority first) to ensure higher priority files come last
        if a.priority != b.priority {
            a.priority.cmp(&b.priority)
        } else {
            // If priorities are equal, sort by git commit time (more recent first)
            let time_a = git_times
                .as_ref()
                .and_then(|times| times.get(&a.rel_path))
                .copied()
                .unwrap_or(0);
            let time_b = git_times
                .as_ref()
                .and_then(|times| times.get(&b.rel_path))
                .copied()
                .unwrap_or(0);

            // If times are equal, sort by path for stability
            if time_a == time_b {
                a.rel_path.cmp(&b.rel_path)
            } else {
                time_b.cmp(&time_a)
            }
        }
    });

    let max_size = config.max_size.unwrap_or(usize::MAX);
    tracing::debug!("Max size limit: {}", max_size);
    let mut output_content = String::new();
    let mut current_size = 0;

    for entry in sorted_results.iter() {
        let model = config.tokenizer_model.as_deref().unwrap_or("openai");
        let entry_header = format!(">>>> {}\n", entry.rel_path);

        // Calculate total entry size including header and content
        let content_with_newline = format!("{}\n", entry.content);
        if config.token_mode {
            // TOKEN-MODE truncation logic
            let header_tokens = model_manager::tokenize(&entry_header, model)?;
            let content_tokens = model_manager::tokenize(&content_with_newline, model)?;
            let total_tokens_needed = header_tokens.len() + content_tokens.len();
            tracing::debug!(
                "Processing file {} in token mode - header tokens: {}, content tokens: {}, total needed: {}, current: {}, max: {}",
                entry.rel_path,
                header_tokens.len(),
                content_tokens.len(),
                total_tokens_needed,
                current_size,
                max_size
            );

            if current_size + total_tokens_needed > max_size {
                // Not enough space for the entire file
                if current_size + header_tokens.len() > max_size {
                    // Can't even fit the header
                    tracing::debug!("Cannot fit header tokens, breaking");
                    break;
                }

                // Push header
                output_content.push_str(&entry_header);

                // Slice content tokens and decode
                let available_tokens = max_size - (current_size + header_tokens.len());
                tracing::debug!(
                    "Truncating content - available tokens: {}, content tokens: {}",
                    available_tokens,
                    content_tokens.len()
                );
                if available_tokens > 0 {
                    let truncated_tokens =
                        &content_tokens[..available_tokens.min(content_tokens.len())];
                    let truncated_str = model_manager::decode_tokens(truncated_tokens, model)?;
                    output_content.push_str(&truncated_str);
                }
                break;
            } else {
                // Fits fully
                tracing::debug!("File fits entirely within token limit");
                output_content.push_str(&entry_header);
                output_content.push_str(&content_with_newline);
                current_size += total_tokens_needed;
            }
        } else {
            // BYTE-MODE truncation logic
            let header_size = entry_header.len();
            let content_size = content_with_newline.len();
            tracing::debug!(
                "Processing file {} in byte mode - header size: {}, content size: {}, total needed: {}, current: {}, max: {}",
                entry.rel_path,
                header_size,
                content_size,
                header_size + content_size,
                current_size,
                max_size
            );

            // If we can't fit the entire entry, truncate it
            if current_size + header_size + content_size > max_size {
                // Can't even fit the header
                if current_size + header_size > max_size {
                    tracing::debug!("Cannot fit header bytes, breaking");
                    break;
                }

                // Push header
                output_content.push_str(&entry_header);
                current_size += header_size;

                // Calculate remaining space for content
                let available_for_content = max_size.saturating_sub(current_size);
                tracing::debug!(
                    "Truncating content - available bytes: {}, content size: {}",
                    available_for_content,
                    content_size
                );
                if available_for_content > 0 {
                    // Take only what we can fit
                    let truncated = &content_with_newline[..available_for_content];
                    output_content.push_str(truncated);
                }
                break;
            } else {
                // Everything fits
                tracing::debug!("File fits entirely within byte limit");
                output_content.push_str(&entry_header);
                output_content.push_str(&content_with_newline);
                current_size += header_size + content_size;
            }
        }
    }
    tracing::debug!("Final output size: {}", output_content.len());
    output_chunks.push(output_content);

    Ok(())
}
