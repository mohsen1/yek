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
    pub file_index: usize,
    pub rel_path: String,
    pub content: String,
    pub token_count: Option<usize>,
}

pub fn process_files_parallel(base_dir: &Path, config: &YekConfig) -> Result<Vec<ProcessedFile>> {
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
    let processed_files = Arc::new(Mutex::new(HashSet::new()));
    let file_counter = Arc::new(Mutex::new(0_usize));

    let walker = WalkBuilder::new(&*base_dir)
        .hidden(true)
        .git_ignore(true)
        .follow_links(false)
        .standard_filters(true)
        .require_git(false)
        .build_parallel();

    let config = Arc::new(config.clone());
    let git_times = Arc::new(git_times);

    walker.run(|| {
        let tx = tx.clone();
        let config = Arc::clone(&config);
        let processed_files = Arc::clone(&processed_files);
        let git_times = Arc::clone(&git_times);
        let file_counter = Arc::clone(&file_counter);
        let base_dir = base_dir.to_path_buf();

        Box::new(move |entry_result| {
            let entry = match entry_result {
                Ok(e) => e,
                Err(_) => return WalkState::Continue,
            };

            if !entry.file_type().map(|ft| ft.is_file()).unwrap_or(false) {
                return WalkState::Continue;
            }

            let path = entry.path();
            let rel_path = normalize_path(path, &base_dir);

            // Check if file has been processed
            {
                let mut processed = processed_files.lock().unwrap();
                if !processed.insert(rel_path.clone()) {
                    return WalkState::Continue;
                }
            }

            // Check ignore patterns - convert all patterns to regex first
            let should_ignore = config.ignore_patterns.iter().any(|p| {
                let pattern = glob_to_regex(p);
                if let Ok(re) = Regex::new(&pattern) {
                    re.is_match(&rel_path)
                } else {
                    false
                }
            });

            if should_ignore {
                debug!("Skipping {} - matched ignore pattern", rel_path);
                return WalkState::Continue;
            }

            // Check binary files with user extensions
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

            // Read file content
            let content = match std::fs::read_to_string(path) {
                Ok(c) => c,
                Err(_) => return WalkState::Continue,
            };

            // Calculate base priority from rules
            let mut priority = get_file_priority(&rel_path, &config.priority_rules);

            // Add git recentness boost if available
            if let Some(git_times) = &*git_times {
                if let Some(&commit_time) = git_times.get(&rel_path) {
                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs();
                    let age = now.saturating_sub(commit_time);
                    let boost = match age {
                        a if a < 86400 => 30,   // Last 24 hours
                        a if a < 604800 => 20,  // Last week
                        a if a < 2592000 => 10, // Last month
                        _ => 0,
                    };
                    priority += boost;
                }
            }

            // Get file index for stable sorting
            let file_index = {
                let mut counter = file_counter.lock().unwrap();
                *counter += 1;
                *counter - 1
            };

            let processed = ProcessedFile {
                priority,
                file_index,
                rel_path,
                content,
                token_count: None,
            };

            let _ = tx.send(processed);
            WalkState::Continue
        })
    });

    drop(tx); // Close sender channel when all walkers are done

    // Collect results without sorting
    let results: Vec<_> = rx.iter().collect();

    Ok(results)
}
