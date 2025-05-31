use crate::{config::YekConfig, priority::get_file_priority, Result};
use content_inspector::{inspect, ContentType};
use glob::glob;
use ignore::gitignore::GitignoreBuilder;
use path_slash::PathBufExt;
use rayon::prelude::*;
use std::{
    collections::HashMap,
    fs,
    path::Path,
    sync::{mpsc, Arc},
};
use tracing::debug;

#[derive(Debug, Clone)]
pub struct ProcessedFile {
    pub priority: i32,
    pub file_index: usize,
    pub rel_path: String,
    pub content: String,
}

/// Process a single file specified directly (not from glob expansion), preserving relative path from CWD.
fn process_single_file_direct(
    file_path: &Path,
    config: &YekConfig,
    boost_map: &HashMap<String, i32>,
) -> Result<Vec<ProcessedFile>> {
    // Use current working directory as base to preserve relative path structure
    let base_dir = std::env::current_dir().unwrap_or_else(|_| Path::new(".").to_path_buf());
    
    // Convert to absolute path first to handle relative inputs properly
    let abs_file_path = if file_path.is_absolute() {
        file_path.to_path_buf()
    } else {
        base_dir.join(file_path)
    };
    
    let rel_path = normalize_path(&abs_file_path, &base_dir);

    // Build the gitignore
    let mut gitignore_builder = GitignoreBuilder::new(&base_dir);
    for pattern in &config.ignore_patterns {
        gitignore_builder.add_line(None, pattern)?;
    }

    // If there is a .gitignore in this folder, add it last so its "!" lines override prior patterns
    let gitignore_file = base_dir.join(".gitignore");
    if gitignore_file.exists() {
        gitignore_builder.add(&gitignore_file);
    }

    let gitignore = gitignore_builder.build()?;
    if gitignore.matched(&abs_file_path, false).is_ignore() {
        debug!("Skipping ignored file: {rel_path}");
        return Ok(Vec::new());
    }

    let mut processed_files = Vec::new();

    match fs::read(&abs_file_path) {
        Ok(content) => {
            if inspect(&content) == ContentType::BINARY {
                debug!("Skipping binary file: {rel_path}");
            } else {
                let rule_priority = get_file_priority(&rel_path, &config.priority_rules);
                let boost = boost_map.get(&rel_path).copied().unwrap_or(0);
                let combined_priority = rule_priority + boost;

                processed_files.push(ProcessedFile {
                    priority: combined_priority,
                    file_index: 0, // For a single file, the index is always 0
                    rel_path,
                    content: String::from_utf8_lossy(&content).to_string(),
                });
            }
        }
        Err(e) => {
            debug!("Failed to read {rel_path}: {e}");
        }
    }

    Ok(processed_files)
}

/// Process a single file, checking ignore patterns and reading its contents.
fn process_single_file(
    file_path: &Path,
    config: &YekConfig,
    boost_map: &HashMap<String, i32>,
) -> Result<Vec<ProcessedFile>> {
    let base_dir = file_path.parent().unwrap_or(Path::new(""));
    let rel_path = normalize_path(file_path, base_dir);

    // Build the gitignore
    let mut gitignore_builder = GitignoreBuilder::new(&base_dir);
    for pattern in &config.ignore_patterns {
        gitignore_builder.add_line(None, pattern)?;
    }

    // If there is a .gitignore in this folder, add it last so its "!" lines override prior patterns
    let gitignore_file = base_dir.join(".gitignore");
    if gitignore_file.exists() {
        gitignore_builder.add(&gitignore_file);
    }

    let gitignore = gitignore_builder.build()?;
    if gitignore.matched(file_path, false).is_ignore() {
        debug!("Skipping ignored file: {rel_path}");
        return Ok(Vec::new());
    }

    let mut processed_files = Vec::new();

    match fs::read(file_path) {
        Ok(content) => {
            if inspect(&content) == ContentType::BINARY {
                debug!("Skipping binary file: {rel_path}");
            } else {
                let rule_priority = get_file_priority(&rel_path, &config.priority_rules);
                let boost = boost_map.get(&rel_path).copied().unwrap_or(0);
                let combined_priority = rule_priority + boost;

                processed_files.push(ProcessedFile {
                    priority: combined_priority,
                    file_index: 0, // For a single file, the index is always 0
                    rel_path,
                    content: String::from_utf8_lossy(&content).to_string(),
                });
            }
        }
        Err(e) => {
            debug!("Failed to read {rel_path}: {e}");
        }
    }

    Ok(processed_files)
}

/// Walk files in parallel (if a directory is given), skipping ignored paths,
/// then read each file's contents in a separate thread.
/// Return the resulting `ProcessedFile` objects.
pub fn process_files_parallel(
    base_path: &Path,
    config: &YekConfig,
    boost_map: &HashMap<String, i32>,
) -> Result<Vec<ProcessedFile>> {
    let path_str = base_path.to_string_lossy();
    
    // Check if this is a direct file path (not a glob pattern)
    let is_direct_file = base_path.exists() && base_path.is_file();
    
    if is_direct_file {
        // This is a direct file argument, preserve the path structure relative to CWD
        return process_single_file_direct(base_path, config, boost_map);
    }
    
    // Expand globs into a list of paths
    let mut expanded_paths = Vec::new();
    for entry in glob(&path_str)? {
        match entry {
            Ok(path) => expanded_paths.push(path),
            Err(e) => debug!("Glob entry error: {:?}", e),
        }
    }

    // If it's a single file (from glob expansion), process it with parent dir as base
    if expanded_paths.len() == 1 && expanded_paths[0].is_file() {
        return process_single_file(&expanded_paths[0], config, boost_map);
    }

    // Iterate over expanded paths, handling files and directories
    let mut all_processed_files = Vec::new();
    for path in expanded_paths {
        if path.is_file() {
            all_processed_files.extend(process_single_file(&path, config, boost_map)?);
        } else if path.is_dir() {
            // For directories, use the original recursive logic
            all_processed_files.extend(process_files_parallel_internal(&path, config, boost_map)?);
        }
    }

    Ok(all_processed_files)
}

/// Internal function to handle directory recursion (separated for clarity)
fn process_files_parallel_internal(
    base_path: &Path,
    config: &YekConfig,
    boost_map: &HashMap<String, i32>,
) -> Result<Vec<ProcessedFile>> {
    // It's a directory, so walk it
    let mut walk_builder = ignore::WalkBuilder::new(base_path);

    // Standard filters + no follow symlinks
    walk_builder
        .follow_links(false)
        .standard_filters(true)
        .require_git(false);

    // Build the gitignore
    let mut gitignore_builder = GitignoreBuilder::new(base_path);
    // Add our custom patterns first
    for pattern in &config.ignore_patterns {
        gitignore_builder.add_line(None, pattern)?;
    }

    // If there is a .gitignore in this folder, add it last so its "!" lines override prior patterns
    let gitignore_file = base_path.join(".gitignore");
    if gitignore_file.exists() {
        gitignore_builder.add(&gitignore_file);
    }

    let gitignore = Arc::new(gitignore_builder.build()?); // Propagate error here

    // This channel will carry (path, rel_path) to the processing thread
    let (processed_files_tx, processed_files_rx) = mpsc::channel::<(std::path::PathBuf, String)>();

    // Processing happens on a dedicated thread, to keep from blocking the main walker
    let process_thread = std::thread::spawn({
        let priority_rules = config.priority_rules.clone();
        let boost_map = boost_map.clone();
        move || {
            let mut processed = Vec::new();
            for (path, rel_path) in processed_files_rx {
                // Read entire file
                match fs::read(&path) {
                    Ok(content) => {
                        // Check if it's binary quickly
                        if inspect(&content) == ContentType::BINARY {
                            debug!("Skipping binary file: {rel_path}");
                            continue;
                        }
                        // Compute priority
                        let rule_priority = get_file_priority(&rel_path, &priority_rules);
                        let boost = boost_map.get(&rel_path).copied().unwrap_or(0);
                        let combined = rule_priority + boost;
                        processed.push(ProcessedFile {
                            priority: combined,
                            file_index: 0, // assigned later
                            rel_path,
                            content: String::from_utf8_lossy(&content).to_string(),
                        });
                    }
                    Err(e) => {
                        debug!("Failed to read {rel_path}: {e}");
                    }
                }
            }
            processed
        }
    });

    // Use ignore's parallel walker to skip ignored files
    let base_cloned = base_path.to_owned();
    let walker_tx = processed_files_tx.clone();

    // Now build the walker (no .gitignore custom filename)
    walk_builder.build_parallel().run(move || {
        let base_dir = base_cloned.clone();
        let processed_files_tx = walker_tx.clone();
        let gitignore = Arc::clone(&gitignore);

        Box::new(move |entry| {
            let entry = match entry {
                Ok(e) => e,
                Err(_) => return ignore::WalkState::Continue,
            };
            // Only process files
            if !entry.file_type().is_some_and(|ft| ft.is_file()) {
                return ignore::WalkState::Continue;
            }

            let path = entry.path().to_path_buf();
            let rel_path = normalize_path(&path, &base_dir);

            // If gitignore says skip, we do not even read
            if gitignore.matched(&path, false).is_ignore() {
                debug!("Skipping ignored file: {rel_path}");
                return ignore::WalkState::Continue;
            }

            // Otherwise we send to processing thread
            processed_files_tx.send((path, rel_path)).ok();
            ignore::WalkState::Continue
        })
    });

    // Drop the sender so the thread can end
    drop(processed_files_tx);

    // Join the processing thread
    let mut processed_files = process_thread.join().unwrap();

    // Now assign file_index within each priority group
    let mut counters = HashMap::new();
    for f in &mut processed_files {
        let ctr = counters.entry(f.priority).or_insert(0);
        f.file_index = *ctr;
        *ctr += 1;
    }

    if config.debug {
        debug!(
            "Processed {} files in parallel for base_path: {}",
            processed_files.len(),
            base_path.display()
        );
    }

    // Sort by priority desc, then file_index
    processed_files.par_sort_by(|a, b| {
        a.priority
            .cmp(&b.priority)
            .reverse()
            .then_with(|| a.file_index.cmp(&b.file_index))
    });

    Ok(processed_files)
}

/// Create a relative, slash-normalized path
pub fn normalize_path(path: &Path, base: &Path) -> String {
    path.strip_prefix(base)
        .unwrap_or(path)
        .to_path_buf()
        .to_slash()
        .unwrap_or_default()
        .to_string()
}
