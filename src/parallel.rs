use crate::is_text_file;
use crate::{get_file_priority, PriorityPattern, YekConfig};
use anyhow::Result;
use crossbeam::channel::{bounded, Receiver, Sender};
use ignore::{gitignore::GitignoreBuilder, WalkBuilder};
use num_cpus::get;
use regex::Regex;
use std::collections::HashMap;
use std::fs;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use std::thread;
use tracing::{debug, info};

/// Represents a chunk of text read from one file
#[derive(Debug)]
pub struct FileChunk {
    pub priority: i32,
    pub file_index: usize,
    pub part_index: usize,
    pub rel_path: String,
    pub content: String,
}

/// File entry with priority for sorting
#[derive(Debug, Clone)]
struct FileEntry {
    path: PathBuf,
    priority: i32,
    file_index: usize,
}

/// Reads a file and determines if it's likely binary by checking for null bytes
fn is_likely_binary(path: &Path) -> Result<bool> {
    let f = fs::File::open(path)?;
    let mut reader = BufReader::new(f);
    let mut buf = [0; 4096];
    let n = reader.read(&mut buf)?;
    Ok(buf[..n].contains(&0))
}

/// Reads and chunks a single file, sending chunks through the channel
fn read_and_send_chunks(
    file_entry: FileEntry,
    base_path: &Path,
    tx: &Sender<FileChunk>,
    max_size: usize,
) -> Result<()> {
    // Skip if binary
    if is_likely_binary(&file_entry.path)? {
        return Ok(());
    }

    // Read file content
    let content = fs::read_to_string(&file_entry.path)?;
    if content.is_empty() {
        return Ok(());
    }

    // Get relative path for display
    let rel_path = file_entry
        .path
        .strip_prefix(base_path)
        .unwrap_or(&file_entry.path)
        .to_string_lossy()
        .into_owned();

    // If smaller than max_size, send as single chunk
    if content.len() <= max_size {
        let chunk = FileChunk {
            priority: file_entry.priority,
            file_index: file_entry.file_index,
            part_index: 0,
            rel_path,
            content,
        };
        tx.send(chunk).ok();
        return Ok(());
    }

    // Otherwise split into chunks
    let mut start = 0;
    let mut part_index = 0;
    let bytes = content.as_bytes();

    while start < bytes.len() {
        let end = (start + max_size).min(bytes.len());
        let slice = &bytes[start..end];
        let chunk_str = String::from_utf8_lossy(slice).into_owned();

        let chunk = FileChunk {
            priority: file_entry.priority,
            file_index: file_entry.file_index,
            part_index,
            rel_path: rel_path.clone(),
            content: chunk_str,
        };

        tx.send(chunk).ok();
        start = end;
        part_index += 1;
    }

    Ok(())
}

/// Main parallel processing function that coordinates workers and aggregator
pub fn process_files_parallel(
    base_dir: &Path,
    max_size: usize,
    output_dir: &Path,
    config: Option<&YekConfig>,
    ignore_patterns: &[Regex],
    priority_list: &[PriorityPattern],
    recentness_boost: Option<&HashMap<String, i32>>,
) -> Result<()> {
    // Create output directory
    fs::create_dir_all(output_dir)?;

    // Collect and sort files by priority
    let files = collect_files(
        base_dir,
        config,
        ignore_patterns,
        priority_list,
        recentness_boost,
    )?;
    if files.is_empty() {
        return Ok(());
    }

    // Create channels for worker→aggregator communication
    let (tx, rx) = bounded(256);

    // Spawn aggregator thread
    let output_dir = output_dir.to_path_buf();
    let aggregator_handle = thread::spawn(move || aggregator_loop(rx, output_dir));

    // Spawn worker threads
    let num_threads = get();
    let chunk_size = files.len().div_ceil(num_threads);
    let mut handles = Vec::new();

    for chunk in files.chunks(chunk_size) {
        let chunk_files = chunk.to_vec();
        let sender = tx.clone();
        let base_path = base_dir.to_path_buf();

        let handle = thread::spawn(move || -> Result<()> {
            for file_entry in chunk_files {
                read_and_send_chunks(file_entry, &base_path, &sender, max_size)?;
            }
            Ok(())
        });
        handles.push(handle);
    }

    // Drop original sender
    drop(tx);

    // Wait for workers
    for handle in handles {
        handle.join().unwrap()?;
    }

    // Wait for aggregator
    aggregator_handle.join().unwrap()?;

    Ok(())
}

/// Collects files from directory respecting .gitignore and sorts by priority
fn collect_files(
    base_dir: &Path,
    config: Option<&YekConfig>,
    ignore_patterns: &[Regex],
    priority_list: &[PriorityPattern],
    recentness_boost: Option<&HashMap<String, i32>>,
) -> Result<Vec<FileEntry>> {
    // Build gitignore matcher
    let mut builder = GitignoreBuilder::new(base_dir);
    let gitignore_path = base_dir.join(".gitignore");
    if gitignore_path.exists() {
        builder.add(&gitignore_path);
    }
    let gitignore = builder
        .build()
        .unwrap_or_else(|_| GitignoreBuilder::new(base_dir).build().unwrap());

    let mut builder = WalkBuilder::new(base_dir);
    builder.follow_links(false).standard_filters(true);

    let mut results = Vec::new();
    let mut file_index = 0;

    for entry in builder.build().flatten() {
        if entry.file_type().is_some_and(|ft| ft.is_file()) {
            let path = entry.path().to_path_buf();
            let rel_path = path.strip_prefix(base_dir).unwrap_or(&path);
            let rel_str = rel_path.to_string_lossy();

            // Skip if matched by gitignore
            #[cfg(windows)]
            let gitignore_path = rel_path
                .to_str()
                .map(|s| s.replace('\\', "/"))
                .map(PathBuf::from)
                .unwrap_or(rel_path.to_path_buf());
            #[cfg(not(windows))]
            let gitignore_path = rel_path.to_path_buf();

            if gitignore.matched(&gitignore_path, false).is_ignore() {
                debug!("Skipping {} - matched by gitignore", rel_str);
                continue;
            }

            // Skip if matched by our ignore patterns
            let mut skip = false;
            for pat in ignore_patterns {
                if pat.is_match(&rel_str) {
                    debug!("Skipping {} - matched ignore pattern", rel_str);
                    skip = true;
                    break;
                }
            }
            if skip {
                continue;
            }

            // Skip binary files
            if let Some(cfg) = config {
                if !is_text_file(&path, &cfg.binary_extensions) {
                    debug!("Skipping binary file: {}", rel_str);
                    continue;
                }
            } else if !is_text_file(&path, &[]) {
                debug!("Skipping binary file: {}", rel_str);
                continue;
            }

            // Calculate priority score
            let mut priority = get_file_priority(&rel_str, ignore_patterns, priority_list);

            // Apply git recentness boost
            if let Some(boost_map) = recentness_boost {
                if let Some(boost) = boost_map.get(&rel_str.to_string()) {
                    priority += *boost;
                }
            }

            results.push(FileEntry {
                path,
                priority,
                file_index,
            });
            file_index += 1;
        }
    }

    // Sort by priority (ascending) so higher priority files come last
    results.sort_by(|a, b| {
        // First sort by priority (ascending)
        let p = a.priority.cmp(&b.priority);
        if p != std::cmp::Ordering::Equal {
            return p;
        }
        // If priorities are equal, sort by Git boost (ascending)
        if let Some(boost_map) = recentness_boost {
            let a_boost = boost_map
                .get(&a.path.to_string_lossy().to_string())
                .unwrap_or(&0);
            let b_boost = boost_map
                .get(&b.path.to_string_lossy().to_string())
                .unwrap_or(&0);
            return a_boost.cmp(b_boost); // Lower boost (older files) come first
        }
        std::cmp::Ordering::Equal
    });
    Ok(results)
}

/// Receives chunks from workers and writes them to files
fn aggregator_loop(rx: Receiver<FileChunk>, output_dir: PathBuf) -> Result<()> {
    fs::create_dir_all(&output_dir)?;

    let mut all_chunks = Vec::new();
    while let Ok(chunk) = rx.recv() {
        all_chunks.push(chunk);
    }

    all_chunks.sort_by(|a, b| {
        let p = a.priority.cmp(&b.priority);
        if p != std::cmp::Ordering::Equal {
            return p;
        }
        let f = a.file_index.cmp(&b.file_index);
        if f != std::cmp::Ordering::Equal {
            return f;
        }
        a.part_index.cmp(&b.part_index)
    });

    let mut current_chunk = String::new();
    let current_chunk_index = 0;

    for chunk in all_chunks {
        let mut content = String::new();
        content.push_str(&format!(">>>> {}\n", chunk.rel_path));
        content.push_str(&chunk.content);
        content.push_str("\n\n");
        current_chunk.push_str(&content);
    }

    if !current_chunk.is_empty() {
        let out_path = output_dir.join(format!("chunk-{}.txt", current_chunk_index));
        fs::write(&out_path, &current_chunk)?;
        info!(
            "Written chunk {} with {} lines.",
            current_chunk_index,
            current_chunk.lines().count()
        );
    }

    Ok(())
}
