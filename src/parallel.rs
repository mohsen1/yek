use crate::{get_file_priority, is_text_file, PriorityPattern, YekConfig};
use anyhow::Result;
use crossbeam::channel::{bounded, Receiver, Sender};
use ignore::{gitignore::GitignoreBuilder, WalkBuilder};
use num_cpus::get;
use regex::Regex;
use std::collections::HashMap;
use std::fs;
use std::io::Read;
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

/// Main parallel processing function that coordinates workers and aggregator
pub const DEFAULT_CHANNEL_CAPACITY: usize = 1024;
pub const PARALLEL_THRESHOLD: usize = 10; // Only parallelize if more than 10 files

pub fn process_files_parallel(
    base_dir: &Path,
    max_size: usize,
    output_dir: &Path,
    config: Option<&YekConfig>,
    ignore_patterns: &[Regex],
    priority_list: &[PriorityPattern],
    recentness_boost: Option<&HashMap<String, i32>>,
) -> Result<()> {
    fs::create_dir_all(output_dir)?;

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

    // For small sets of files, process sequentially
    if files.len() <= PARALLEL_THRESHOLD {
        debug!("Processing {} files sequentially", files.len());
        let mut current_chunk = String::new();
        let mut current_chunk_size = 0;
        let mut current_chunk_index = 0;

        for file in files {
            let content = match fs::read_to_string(&file.path) {
                Ok(c) => c,
                Err(e) => {
                    debug!("Failed to read {}: {}", file.path.display(), e);
                    continue;
                }
            };

            if content.is_empty() {
                continue;
            }

            let rel_path = file
                .path
                .strip_prefix(base_dir)
                .unwrap_or(&file.path)
                .to_string_lossy()
                .into_owned();

            let chunk_str = format!(">>>> {}\n{}\n\n", rel_path, content);
            let chunk_size = chunk_str.len();

            // Write chunk if buffer would exceed size
            if current_chunk_size + chunk_size > max_size {
                write_chunk_to_file(output_dir, current_chunk_index, &current_chunk)?;
                current_chunk.clear();
                current_chunk_size = 0;
                current_chunk_index += 1;
            }

            current_chunk.push_str(&chunk_str);
            current_chunk_size += chunk_size;
        }

        // Write final chunk if any content remains
        if !current_chunk.is_empty() {
            write_chunk_to_file(output_dir, current_chunk_index, &current_chunk)?;
        }

        return Ok(());
    }

    // For larger sets, process in parallel
    debug!("Processing {} files in parallel", files.len());

    let channel_capacity = config
        .and_then(|c| c.channel_capacity)
        .unwrap_or(DEFAULT_CHANNEL_CAPACITY);

    // Create channels for worker→aggregator communication
    let (tx, rx) = bounded(channel_capacity);

    // Spawn aggregator thread
    let output_dir = output_dir.to_path_buf();
    let aggregator_handle = thread::spawn(move || aggregator_loop(rx, output_dir, max_size));

    // Spawn worker threads - use fewer threads for smaller workloads
    let num_threads = if files.len() < 4 { 1 } else { get() };
    let chunk_size = files.len().div_ceil(num_threads);
    let mut handles = Vec::new();

    for chunk in files.chunks(chunk_size) {
        let chunk_files = chunk.to_vec();
        let sender = tx.clone();
        let base_path = base_dir.to_path_buf();

        let handle = thread::spawn(move || -> Result<()> {
            for file_entry in chunk_files {
                read_and_send_chunks(&base_path, file_entry, max_size, &sender)?;
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

/// Reads and chunks a single file, sending chunks through the channel
fn read_and_send_chunks(
    base_path: &Path,
    file_entry: FileEntry,
    max_size: usize,
    tx: &Sender<FileChunk>,
) -> Result<()> {
    let mut file = fs::File::open(&file_entry.path)?;
    let rel_path = file_entry
        .path
        .strip_prefix(base_path)
        .unwrap_or(&file_entry.path)
        .to_string_lossy()
        .into_owned();

    // Read file content in chunks to avoid loading entire file
    let mut total_buf = Vec::new();
    file.read_to_end(&mut total_buf)?;

    if total_buf.is_empty() {
        return Ok(());
    }

    // If total size <= max_size, send it as single chunk
    if total_buf.len() <= max_size {
        let chunk_content = String::from_utf8_lossy(&total_buf).to_string();
        let fc = FileChunk {
            priority: file_entry.priority,
            file_index: file_entry.file_index,
            part_index: 0,
            rel_path,
            content: chunk_content,
        };
        tx.send(fc)?;
        return Ok(());
    }

    // Otherwise break into multiple parts
    let mut start = 0;
    let mut part_index = 0;
    while start < total_buf.len() {
        let end = (start + max_size).min(total_buf.len());
        let slice = &total_buf[start..end];
        let chunk_str = String::from_utf8_lossy(slice).to_string();

        let fc = FileChunk {
            priority: file_entry.priority,
            file_index: file_entry.file_index,
            part_index,
            rel_path: format!("{}:part{}", rel_path, part_index),
            content: chunk_str,
        };
        tx.send(fc)?;
        start = end;
        part_index += 1;
    }
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
            let rel_str = rel_path
                .to_str()
                .map(|s| s.replace('\\', "/"))
                .unwrap_or_else(|| rel_str.to_string());

            #[cfg(not(windows))]
            let rel_str = rel_str.to_string();

            // Skip if matched by gitignore
            #[cfg(windows)]
            let gitignore_path = rel_str.clone();

            #[cfg(not(windows))]
            let gitignore_path = rel_str.clone();

            if gitignore.matched(&path, path.is_dir()).is_ignore() {
                continue;
            }

            // Skip if matched by custom ignore patterns
            if ignore_patterns.iter().any(|p| p.is_match(&rel_str)) {
                continue;
            }

            // Skip binary files
            if !is_text_file(
                &path,
                config.map(|c| &c.binary_extensions[..]).unwrap_or(&[]),
            ) {
                continue;
            }

            // Calculate priority
            let mut priority = get_file_priority(&rel_str, ignore_patterns, priority_list);

            // Apply recentness boost if available
            if let Some(boost_map) = recentness_boost {
                if let Some(boost) = boost_map.get(&gitignore_path) {
                    priority += boost;
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
fn aggregator_loop(rx: Receiver<FileChunk>, output_dir: PathBuf, max_size: usize) -> Result<()> {
    // Collect chunks first to maintain priority order
    let mut all_chunks = Vec::new();
    while let Ok(chunk) = rx.recv() {
        all_chunks.push(chunk);
    }

    // Sort chunks by priority, file index, and part index
    all_chunks.sort_by(|a, b| {
        a.priority
            .cmp(&b.priority)
            .then(a.file_index.cmp(&b.file_index))
            .then(a.part_index.cmp(&b.part_index))
    });

    let mut current_chunk = String::new();
    let mut current_chunk_size = 0;
    let mut current_chunk_index = 0;
    let mut current_priority = None;

    // Process chunks in sorted order
    for chunk in all_chunks {
        let chunk_str = format!(">>>> {}\n{}\n\n", chunk.rel_path, chunk.content);
        let chunk_size = chunk_str.len();

        let should_start_new_chunk = current_chunk_size + chunk_size > max_size
            || (current_priority.is_some() && current_priority.unwrap() != chunk.priority);

        if should_start_new_chunk && !current_chunk.is_empty() {
            write_chunk_to_file(&output_dir, current_chunk_index, &current_chunk)?;
            current_chunk.clear();
            current_chunk_size = 0;
            current_chunk_index += 1;
        }

        current_chunk.push_str(&chunk_str);
        current_chunk_size += chunk_size;
        current_priority = Some(chunk.priority);
    }

    // Write final chunk if any content remains
    if !current_chunk.is_empty() {
        write_chunk_to_file(&output_dir, current_chunk_index, &current_chunk)?;
    }

    Ok(())
}

fn write_chunk_to_file(output_dir: &Path, index: usize, content: &str) -> Result<()> {
    let chunk_path = output_dir.join(format!("chunk-{}.txt", index));
    fs::write(&chunk_path, content)?;
    info!(
        "Written chunk {} with {} lines.",
        index,
        content.lines().count()
    );
    Ok(())
}
