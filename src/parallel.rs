use crate::{
    debug_file, get_file_priority, is_text_file, normalize_path, write_debug_to_file,
    PriorityPattern, Result, YekConfig,
};
use crossbeam::channel::{bounded, Receiver, Sender};
use ignore::{gitignore::GitignoreBuilder, WalkBuilder};
use num_cpus::get;
use regex::Regex;
use std::{
    collections::HashMap,
    fs,
    io::Read,
    path::{Path, PathBuf},
    thread,
};
use tracing::{debug, info};

/// Default chunk size in bytes
pub const CHUNK_SIZE_BYTES: usize = 1024;
/// Minimum content size that triggers chunking
pub const MIN_CONTENT_SIZE: usize = CHUNK_SIZE_BYTES * 2;

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
                Err(_e) => {
                    debug!("Failed to read {}", normalize_path(base_dir, &file.path));
                    continue;
                }
            };

            if content.is_empty() {
                continue;
            }

            let rel_str = normalize_path(base_dir, &file.path);
            let chunk_str = format!(">>>> {}\n{}\n\n", rel_str, content);
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
    _max_size: usize,
    tx: &Sender<FileChunk>,
) -> Result<()> {
    let mut file = fs::File::open(&file_entry.path)?;
    let rel_str = normalize_path(base_path, &file_entry.path);

    // Read file content in chunks to avoid loading entire file
    let mut total_buf = Vec::new();
    file.read_to_end(&mut total_buf)?;

    if total_buf.is_empty() {
        return Ok(());
    }

    // If total size <= max_size, send it as single chunk
    if total_buf.len() <= MIN_CONTENT_SIZE {
        let chunk_content = String::from_utf8_lossy(&total_buf).to_string();
        let fc = FileChunk {
            priority: file_entry.priority,
            file_index: file_entry.file_index,
            part_index: 0,
            rel_path: rel_str.to_string(),
            content: chunk_content,
        };
        tx.send(fc)?;
        return Ok(());
    }

    // Otherwise break into multiple parts using CHUNK_SIZE_BYTES
    let mut start = 0;
    let mut part_index = 0;
    while start < total_buf.len() {
        let end = (start + CHUNK_SIZE_BYTES).min(total_buf.len());
        let slice = &total_buf[start..end];
        let chunk_str = String::from_utf8_lossy(slice).to_string();

        let fc = FileChunk {
            priority: file_entry.priority,
            file_index: file_entry.file_index,
            part_index,
            rel_path: format!("{}:part{}", rel_str, part_index),
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
    builder
        .follow_links(false)
        .standard_filters(true)
        .add_custom_ignore_filename(".gitignore")
        .require_git(false);

    let mut results = Vec::new();
    let mut file_index = 0;

    for entry in builder.build().flatten() {
        if entry.file_type().is_some_and(|ft| ft.is_file()) {
            let path = entry.path().to_path_buf();
            let rel_str = normalize_path(base_dir, &path);
            let rel_path = path.strip_prefix(base_dir).unwrap_or(&path);

            // Skip via .gitignore
            if gitignore.matched(rel_path, false).is_ignore() {
                debug!("Skipping {} - matched by gitignore", rel_str);
                continue;
            }

            // Skip via our ignore regexes
            if ignore_patterns.iter().any(|p| p.is_match(&rel_str)) {
                debug!("Skipping {} - matched ignore pattern", rel_str);
                continue;
            }

            // Check if text or binary
            let user_bin_exts = config
                .as_ref()
                .map(|c| c.binary_extensions.as_slice())
                .unwrap_or(&[]);
            if !is_text_file(&path, user_bin_exts) {
                debug!("Skipping binary file: {}", rel_str);
                continue;
            }

            // Calculate priority
            let mut priority = get_file_priority(&rel_str, priority_list);

            // Add Git-based priority boost if available
            if let Some(boost) = recentness_boost.and_then(|bm| bm.get(&rel_str)) {
                priority += boost;
            }

            results.push(FileEntry {
                path,
                priority,
                file_index,
            });
            file_index += 1;
        }
    }

    // Sort by priority (ascending) and then by file index for deterministic ordering
    results.sort_by(|a, b| {
        a.priority
            .cmp(&b.priority)
            .then_with(|| a.file_index.cmp(&b.file_index))
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

        // Check priority first to avoid unnecessary size checks
        let should_start_new_chunk = (current_priority.is_some()
            && current_priority.unwrap() != chunk.priority)
            || current_chunk_size + chunk_size > max_size;

        if should_start_new_chunk {
            if current_priority.is_some() && current_priority.unwrap() != chunk.priority {
                debug_file!(
                    "Starting new chunk due to priority change: {} -> {}",
                    current_priority.unwrap(),
                    chunk.priority
                );
            } else {
                debug_file!(
                    "Starting new chunk due to size limit: {} + {} > {}",
                    current_chunk_size,
                    chunk_size,
                    max_size
                );
            }
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
