use crate::{config::FullYekConfig, is_text_file, priority::get_file_priority, Result};
use crossbeam::channel::bounded;
use ignore::{WalkBuilder, WalkState};
use regex::Regex;
use std::collections::HashMap;
use std::{
    collections::HashSet,
    path::Path,
    sync::{Arc, Mutex},
    thread,
};
use tracing::debug;

#[derive(Debug)]
pub struct ProcessedFile {
    pub priority: i32,
    pub file_index: usize,
    #[allow(unused)]
    pub rel_path: String,
    pub content: String,
}

pub fn process_files_parallel(
    base_dir: &Path,
    config: &FullYekConfig,
    boost_map: &HashMap<String, i32>,
) -> Result<Vec<ProcessedFile>> {
    let (tx, rx) = bounded(1024);
    let num_threads = num_cpus::get().min(16); // Cap at 16 threads

    let config = Arc::new(config.clone());
    let base_dir = Arc::new(base_dir.to_path_buf());
    let processed_files = Arc::new(Mutex::new(HashSet::new()));
    let boost_map = Arc::new(boost_map.clone());

    // Spawn worker threads
    let mut handles = Vec::new();
    for _ in 0..num_threads {
        let tx = tx.clone();
        let config = Arc::clone(&config);
        let base_dir = Arc::clone(&base_dir);
        let processed_files = Arc::clone(&processed_files);
        let boost_map = Arc::clone(&boost_map);

        let handle = thread::spawn(move || -> Result<()> {
            let file_index = Arc::new(Mutex::new(0_usize));

            // Configure walker for this thread
            let mut builder = WalkBuilder::new(&*base_dir);
            builder
                .hidden(true)
                .git_ignore(true)
                .follow_links(false)
                .standard_filters(true)
                .require_git(false)
                .threads(1); // Single thread per walker

            let walker = builder.build_parallel();

            let file_index = Arc::clone(&file_index);
            walker.run(|| {
                let tx = tx.clone();
                let config = Arc::clone(&config);
                let base_dir = Arc::clone(&base_dir);
                let file_index = Arc::clone(&file_index);
                let processed_files = Arc::clone(&processed_files);
                let boost_map = Arc::clone(&boost_map);

                Box::new(move |entry| {
                    let entry = match entry {
                        Ok(e) => e,
                        Err(_) => return WalkState::Continue,
                    };

                    if !entry.file_type().is_some_and(|ft| ft.is_file()) {
                        return WalkState::Continue;
                    }

                    let path = entry.path().to_path_buf();
                    let rel_path = normalize_path(&path, &base_dir);

                    // Check if file has already been processed
                    {
                        let mut processed = processed_files.lock().unwrap();
                        if !processed.insert(rel_path.clone()) {
                            // File was already processed
                            return WalkState::Continue;
                        }
                    }

                    // Skip files matching ignore patterns from yek co
                    let ignore_patterns = config.ignore_patterns.clone();
                    let is_ignored = ignore_patterns.iter().any(|p| {
                        let str = p.to_string();
                        if let Ok(re) = Regex::new(&str) {
                            re.is_match(&rel_path)
                        } else {
                            false
                        }
                    });
                    if is_ignored {
                        debug!("Skipping {} - matched ignore pattern", rel_path);
                        return WalkState::Continue;
                    }

                    // Skip binary files unless explicitly allowed
                    match is_text_file(&path, &config.binary_extensions) {
                        Ok(is_text) if !is_text => {
                            debug!("Skipping binary file: {}", rel_path);
                            return WalkState::Continue;
                        }
                        Err(_) => return WalkState::Continue,
                        _ => {}
                    }

                    // Read and process file
                    if let Ok(content) = std::fs::read_to_string(&path) {
                        let rule_priority = get_file_priority(&rel_path, &config.priority_rules);
                        let boost = boost_map.get(&rel_path).cloned().unwrap_or(0);
                        let combined_priority = rule_priority + boost;

                        let mut index = file_index.lock().unwrap();
                        let processed = ProcessedFile {
                            priority: combined_priority,
                            file_index: *index,
                            rel_path,
                            content,
                        };

                        if tx.send(processed).is_ok() {
                            *index += 1;
                        }
                    }

                    WalkState::Continue
                })
            });

            Ok(())
        });
        handles.push(handle);
    }

    // Drop original sender
    drop(tx);

    // Collect results
    let mut results = Vec::new();
    while let Ok(processed) = rx.recv() {
        results.push(processed);
    }

    // Wait for all threads
    for handle in handles {
        handle.join().unwrap()?;
    }

    debug!("Processed {} files in parallel", results.len());

    // Sort by priority (ascending) and file index (ascending)
    results.sort_by(|a, b| {
        a.priority
            .cmp(&b.priority)
            .then_with(|| a.file_index.cmp(&b.file_index))
    });

    Ok(results)
}

/// Returns a relative, normalized path string (forward slashes on all platforms).
pub fn normalize_path(path: &Path, base: &Path) -> String {
    // Handle current directory specially
    if path.to_str() == Some(".") {
        return ".".to_string();
    }

    // Resolve both paths to their canonical forms to handle symlinks
    let canonical_path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let canonical_base = base.canonicalize().unwrap_or_else(|_| base.to_path_buf());

    // Attempt to strip the base directory from the file path
    match canonical_path.strip_prefix(&canonical_base) {
        Ok(rel_path) => {
            // Convert to forward slashes and return as relative path
            rel_path.to_string_lossy().replace('\\', "/")
        }
        Err(_) => {
            // Return the absolute path without adding an extra leading slash
            canonical_path.to_string_lossy().replace('\\', "/")
        }
    }
}
