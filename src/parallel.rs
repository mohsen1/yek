use crate::{get_file_priority, glob_to_regex, is_text_file, normalize_path, Result, YekConfig};
use crossbeam::channel::bounded;
use ignore::{WalkBuilder, WalkState};
use num_cpus;
use regex::Regex;
use std::{
    collections::HashSet,
    path::Path,
    sync::{Arc, Mutex},
    thread,
};
use tracing::{debug, info};

#[derive(Debug)]
pub struct ProcessedFile {
    pub priority: i32,
    pub file_index: usize,
    pub rel_path: String,
    pub content: String,
}

pub fn process_files_parallel(base_dir: &Path, config: &YekConfig) -> Result<Vec<ProcessedFile>> {
    let (tx, rx) = bounded(1024);
    let num_threads = num_cpus::get().min(16); // Cap at 16 threads

    let config = Arc::new(config.clone());
    let base_dir = Arc::new(base_dir.to_path_buf());
    let processed_files = Arc::new(Mutex::new(HashSet::new()));

    // Spawn worker threads
    let mut handles = Vec::new();
    for _ in 0..num_threads {
        let tx = tx.clone();
        let config = Arc::clone(&config);
        let base_dir = Arc::clone(&base_dir);
        let processed_files = Arc::clone(&processed_files);

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

                    // Skip files matching ignore patterns from yek.toml
                    if config.ignore_patterns.iter().any(|p| {
                        let pattern = if p.starts_with('^') || p.ends_with('$') {
                            p.to_string()
                        } else {
                            glob_to_regex(p)
                        };
                        if let Ok(re) = Regex::new(&pattern) {
                            re.is_match(&rel_path)
                        } else {
                            false
                        }
                    }) {
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
                        let priority = get_file_priority(&rel_path, &config.priority_rules);

                        let mut index = file_index.lock().unwrap();
                        let processed = ProcessedFile {
                            priority,
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

    info!("Processed {} files in parallel", results.len());

    // Sort by priority (ascending) and file index (ascending)
    results.sort_by(|a, b| {
        a.priority
            .cmp(&b.priority)
            .then_with(|| a.file_index.cmp(&b.file_index))
    });

    Ok(results)
}
