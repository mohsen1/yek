use crate::{config::FullYekConfig, priority::get_file_priority, Result};
use content_inspector::{inspect, ContentType};
use ignore::gitignore::GitignoreBuilder;
use path_slash::PathBufExt;
use rayon::prelude::*;
use std::sync::mpsc;
use std::sync::Arc;
use std::{collections::HashMap, fs, path::Path};
use tracing::debug;

#[derive(Debug, Clone)]
pub struct ProcessedFile {
    pub priority: i32,
    pub file_index: usize,
    pub rel_path: String,
    pub content: String,
}

pub fn process_files_parallel(
    base_dir: &Path,
    config: &FullYekConfig,
    boost_map: &HashMap<String, i32>,
) -> Result<Vec<ProcessedFile>> {
    let mut walk_builder = ignore::WalkBuilder::new(base_dir);

    walk_builder
        .follow_links(false)
        .standard_filters(true)
        .require_git(false);

    // First, build the gitignore rules
    let mut gitignore_builder = GitignoreBuilder::new(base_dir);

    for pattern in &config.ignore_patterns {
        gitignore_builder.add(pattern);
    }

    walk_builder.add_custom_ignore_filename(".gitignore");

    let (processed_files_tx, processed_files_rx) = mpsc::channel::<(std::path::PathBuf, String)>();

    // Create a thread to process files as they are sent
    let process_thread = std::thread::spawn({
        let priority_rules = config.priority_rules.clone();
        let boost_map = boost_map.to_owned();
        move || {
            let mut processed_files = Vec::new();
            for (path, rel_path) in processed_files_rx {
                if let Ok(content) = fs::read(&path) {
                    if inspect(&content) == ContentType::BINARY {
                        debug!("Skipping binary file: {}", rel_path);
                        continue;
                    }

                    let rule_priority = get_file_priority(&rel_path, &priority_rules);
                    let boost = boost_map.get(&rel_path).copied().unwrap_or(0);
                    let combined_priority = rule_priority + boost;

                    processed_files.push(ProcessedFile {
                        priority: combined_priority,
                        file_index: 0, // Placeholder, will be updated later
                        rel_path,
                        content: String::from_utf8_lossy(&content).to_string(),
                    });
                }
            }
            processed_files
        }
    });

    // Send files to the process thread as they are found
    let base_dir = base_dir.to_owned();
    let walker_tx = processed_files_tx.clone();
    let gitignore = Arc::new(gitignore_builder.build()?);
    walk_builder.build_parallel().run(move || {
        let base_dir = base_dir.clone();
        let processed_files_tx = walker_tx.clone();
        let gitignore = Arc::clone(&gitignore);
        Box::new(move |entry| {
            let entry = match entry {
                Ok(entry) => entry,
                Err(_) => return ignore::WalkState::Continue,
            };
            if !entry.file_type().map_or(false, |ft| ft.is_file()) {
                return ignore::WalkState::Continue;
            }

            let path = entry.path().to_path_buf();
            let rel_path = normalize_path(&path, &base_dir);

            // Check gitignore patterns only
            if gitignore.matched(&path, false).is_ignore() {
                debug!("Skipping ignored file: {}", rel_path);
                return ignore::WalkState::Continue;
            }

            processed_files_tx.send((path, rel_path)).unwrap();
            ignore::WalkState::Continue
        })
    });

    // Drop the sender to signal no more files will be sent
    drop(processed_files_tx);

    // Wait for the process thread to finish and get the results
    let mut processed_files = process_thread.join().unwrap();

    // Assign unique file_index within each priority group
    let mut file_index_counters = HashMap::new();
    for file in &mut processed_files {
        let counter = file_index_counters.entry(file.priority).or_insert(0);
        file.file_index = *counter;
        *counter += 1;
    }

    if config.debug {
        debug!("Processed {} files in parallel", processed_files.len());
    }

    processed_files.par_sort_by(|a, b| {
        a.priority
            .cmp(&b.priority)
            .reverse()
            .then_with(|| a.file_index.cmp(&b.file_index))
    });

    Ok(processed_files)
}

/// Returns a relative, normalized path string (forward slashes on all platforms).
pub fn normalize_path(path: &Path, base: &Path) -> String {
    path.strip_prefix(base)
        .unwrap_or(path)
        .to_path_buf()
        .to_slash()
        .unwrap_or_default()
        .to_string()
}
