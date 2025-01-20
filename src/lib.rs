use anyhow::{anyhow, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self};
use std::io::Read;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Command as SysCommand, Stdio};
use tracing::debug;

mod defaults;
mod parallel;

use defaults::{BINARY_FILE_EXTENSIONS, TEXT_FILE_EXTENSIONS};
use parallel::process_files_parallel;

/// Convert a glob pattern to a regex pattern
fn glob_to_regex(pattern: &str) -> String {
    let mut regex = String::with_capacity(pattern.len() * 2);
    let mut chars = pattern.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '*' => {
                if chars.peek() == Some(&'*') {
                    chars.next(); // consume second *
                    regex.push_str(".*");
                } else {
                    regex.push_str("[^/]*");
                }
            }
            '?' => regex.push('.'),
            '.' => regex.push_str("\\."),
            '/' => regex.push('/'),
            '[' => {
                regex.push('[');
                for c in chars.by_ref() {
                    if c == ']' {
                        regex.push(']');
                        break;
                    }
                    regex.push(c);
                }
            }
            '{' => {
                regex.push('(');
                for c in chars.by_ref() {
                    if c == '}' {
                        regex.push(')');
                        break;
                    } else if c == ',' {
                        regex.push('|');
                    } else {
                        regex.push(c);
                    }
                }
            }
            c if c.is_alphanumeric() || c == '_' || c == '-' => regex.push(c),
            c => {
                regex.push('\\');
                regex.push(c);
            }
        }
    }
    regex
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct IgnorePatterns {
    #[serde(default)]
    pub patterns: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriorityRule {
    pub pattern: String,
    pub score: i32,
}

impl PriorityRule {
    #[allow(dead_code)]
    fn matches(&self, path: &str) -> bool {
        if let Ok(re) = Regex::new(&self.pattern) {
            re.is_match(path)
        } else {
            false
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct YekConfig {
    #[serde(default)]
    pub ignore_patterns: Vec<String>,
    #[serde(default)]
    pub priority_rules: Vec<PriorityRule>,
    #[serde(default)]
    pub binary_extensions: Vec<String>,
    #[serde(default)]
    pub max_size: Option<usize>,
    #[serde(default)]
    pub output_dir: Option<PathBuf>,
    #[serde(default)]
    pub stream: bool,
    #[serde(default)]
    pub token_mode: bool,
}

/// Check if file is text by extension or scanning first chunk for null bytes.
pub fn is_text_file(path: &Path, user_binary_extensions: &[String]) -> io::Result<bool> {
    // First check extension - fast path
    if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
        let ext_lc = ext.to_lowercase();
        // If it's in the known text extensions list, it's definitely text
        if TEXT_FILE_EXTENSIONS.contains(&ext_lc.as_str()) {
            return Ok(true);
        }
        // If it's in the binary extensions list (built-in or user-defined), it's definitely binary
        if BINARY_FILE_EXTENSIONS.contains(&ext_lc.as_str())
            || user_binary_extensions
                .iter()
                .any(|e| e.trim_start_matches('.') == ext_lc)
        {
            return Ok(false);
        }
        // Unknown extension - treat as binary
        return Ok(false);
    }

    // No extension - scan content
    let mut file = fs::File::open(path)?;
    let mut buffer = [0; 512];
    let n = file.read(&mut buffer)?;

    // Check for null bytes which typically indicate binary content
    Ok(!buffer[..n].contains(&0))
}

/// Determine final priority of a file by scanning the priority list
/// in descending order of score.
pub fn get_file_priority(path: &str, rules: &[PriorityRule]) -> i32 {
    rules
        .iter()
        .filter_map(|rule| {
            let re = match Regex::new(&rule.pattern) {
                Ok(re) => re,
                Err(_) => return None,
            };
            if re.is_match(path) {
                Some(rule.score)
            } else {
                None
            }
        })
        .max()
        .unwrap_or(0)
}

/// Get the commit time of the most recent change to each file.
/// Returns a map from file path (relative to the repo root) → last commit Unix time.
/// If Git or .git folder is missing, returns None instead of erroring.
pub fn get_recent_commit_times(repo_path: &Path) -> Option<HashMap<String, u64>> {
    // Confirm there's a .git folder
    if !repo_path.join(".git").exists() {
        debug!("No .git directory found, skipping Git-based prioritization");
        return None;
    }

    // Get all files and their timestamps using bash with proper UTF-8 handling
    let output = SysCommand::new("bash")
        .args([
            "-c",
            "export LC_ALL=en_US.UTF-8; export LANG=en_US.UTF-8; \
             git -c core.quotepath=false log \
             --format=%ct \
             --name-only \
             --no-merges \
             --no-renames \
             -- . | tr -cd '[:print:]\n' | iconv -f utf-8 -t utf-8 -c",
        ])
        .current_dir(repo_path)
        .stderr(Stdio::null())
        .output()
        .ok()?;

    if !output.status.success() {
        debug!("Git log command failed, skipping Git-based prioritization");
        return None;
    }

    let mut git_times = HashMap::new();
    let mut current_timestamp = 0_u64;

    // Process output line by line with UTF-8 conversion
    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        if line.is_empty() {
            continue;
        }

        if let Ok(ts) = line.parse::<u64>() {
            current_timestamp = ts;
            debug!("Found timestamp: {}", ts);
        } else {
            debug!("Found file: {} with timestamp {}", line, current_timestamp);
            git_times.insert(line.to_string(), current_timestamp);
        }
    }

    if git_times.is_empty() {
        debug!("No valid timestamps found, skipping Git-based prioritization");
        None
    } else {
        Some(git_times)
    }
}

/// Validate the config object, returning any errors found
#[derive(Debug)]
pub struct ConfigError {
    pub field: String,
    pub message: String,
}

pub fn validate_config(config: &YekConfig) -> Vec<ConfigError> {
    let mut errors = Vec::new();

    // Validate priority rules
    for rule in &config.priority_rules {
        if rule.score < 0 || rule.score > 1000 {
            errors.push(ConfigError {
                field: "priority_rules".to_string(),
                message: format!("Priority score {} must be between 0 and 1000", rule.score),
            });
        }
        if rule.pattern.is_empty() {
            errors.push(ConfigError {
                field: "priority_rules".to_string(),
                message: "Priority rule must have a pattern".to_string(),
            });
        }
        // Validate regex pattern
        if let Err(e) = Regex::new(&rule.pattern) {
            errors.push(ConfigError {
                field: "priority_rules".to_string(),
                message: format!("Invalid regex pattern '{}': {}", rule.pattern, e),
            });
        }
    }

    // Validate ignore patterns
    for pattern in &config.ignore_patterns {
        let regex_pattern = if pattern.starts_with('^') || pattern.ends_with('$') {
            // Already a regex pattern
            pattern.to_string()
        } else {
            // Convert glob pattern to regex
            glob_to_regex(pattern)
        };

        if let Err(e) = Regex::new(&regex_pattern) {
            errors.push(ConfigError {
                field: "ignore_patterns".to_string(),
                message: format!("Invalid pattern '{}': {}", pattern, e),
            });
        }
    }

    // Validate max_size
    if let Some(size) = config.max_size {
        if size == 0 {
            errors.push(ConfigError {
                field: "max_size".to_string(),
                message: "Max size cannot be 0".to_string(),
            });
        }
    }

    // Validate output directory if specified
    if let Some(dir) = &config.output_dir {
        let path = Path::new(dir);
        if path.exists() && !path.is_dir() {
            errors.push(ConfigError {
                field: "output_dir".to_string(),
                message: format!(
                    "Output path '{}' exists but is not a directory",
                    dir.display()
                ),
            });
        }

        if let Err(e) = std::fs::create_dir_all(path) {
            errors.push(ConfigError {
                field: "output_dir".to_string(),
                message: format!("Cannot create output directory '{}': {}", dir.display(), e),
            });
        }
    }

    errors
}

pub const DEFAULT_CHUNK_SIZE: usize = 10 * 1024 * 1024; // 10MB in README

/// Write a single chunk either to stdout or file
fn write_single_chunk(
    content: &str,
    index: usize,
    part_index: Option<usize>,
    out_dir: &Path,
    is_stream: bool,
) -> io::Result<()> {
    if is_stream {
        let mut stdout = io::stdout();
        write!(stdout, "{}", content)?;
        stdout.flush()?;
    } else {
        // Always use chunk index in filename
        let mut file_name = format!("chunk-{}", index);
        if let Some(part_i) = part_index {
            file_name = format!("chunk-{}-part-{}", index, part_i);
        }
        let path = out_dir.join(format!("{}.txt", file_name));
        fs::create_dir_all(path.parent().unwrap())?;
        fs::write(path, content.as_bytes())?;
    }
    Ok(())
}

/// The aggregator that writes chunk-* files or streams to stdout.
fn write_chunks(
    entries: &[(String, String, i32)],
    config: &YekConfig,
    is_stream: bool,
) -> Result<()> {
    debug!("Starting write_chunks with {} entries", entries.len());
    let chunk_size = config.max_size.unwrap_or(DEFAULT_CHUNK_SIZE);
    let token_mode = config.token_mode;

    // Sort entries by priority (ascending)
    let mut sorted_entries = entries.to_vec();
    sorted_entries.sort_by_key(|(_, _, prio)| *prio);

    // For chunk files:
    let out_dir = if !is_stream {
        config
            .output_dir
            .as_ref()
            .expect("output_dir is None but streaming is false")
    } else {
        // dummy
        Path::new(".")
    };
    debug!("Output directory: {:?}", out_dir);

    let mut chunk_idx = 0;
    let mut buffer = String::new();
    let mut used_size = 0_usize;

    // Process each file
    for (rel_path, content, _prio) in sorted_entries {
        debug!("Processing file: {}", rel_path);
        if token_mode {
            // Count tokens
            let tokens: Vec<&str> = content.split_whitespace().collect();
            let file_tokens = tokens.len();
            debug!("Token mode: {} tokens in file", file_tokens);

            // If file exceeds chunk_size by itself, do forced splits
            if file_tokens >= chunk_size {
                debug!("File exceeds chunk size, splitting into multiple chunks");
                // Flush current buffer first
                if !buffer.is_empty() {
                    debug!("Flushing buffer before large file");
                    write_single_chunk(&buffer, chunk_idx, None, out_dir, is_stream)?;
                    buffer.clear();
                    used_size = 0;
                    chunk_idx += 1;
                }

                // Split large file into chunks
                let mut start = 0;
                let mut part = 0;
                while start < file_tokens {
                    let end = (start + chunk_size).min(file_tokens);
                    let chunk_tokens = &tokens[start..end];
                    let chunk_str = format!(
                        "chunk {}\n>>>> {}:part {}\n{}\n",
                        chunk_idx,
                        rel_path,
                        part,
                        chunk_tokens.join(" ")
                    );
                    debug!("Writing large file part {}", part);
                    write_single_chunk(&chunk_str, chunk_idx, Some(part), out_dir, is_stream)?;
                    chunk_idx += 1;
                    part += 1;
                    start = end;
                }
            } else {
                // Small enough to fit in one chunk
                let overhead = 10 + rel_path.len();
                let add_size = file_tokens + overhead;

                if used_size + add_size > chunk_size && !buffer.is_empty() {
                    debug!("Flushing buffer due to size limit");
                    write_single_chunk(&buffer, chunk_idx, None, out_dir, is_stream)?;
                    buffer.clear();
                    used_size = 0;
                    chunk_idx += 1;
                }

                debug!("Adding file to buffer");
                buffer.push_str(&format!("chunk {}\n>>>> {}\n", chunk_idx, rel_path));
                buffer.push_str(&content);
                buffer.push('\n');
                used_size += add_size;
            }
        } else {
            // Byte mode
            let file_len = content.len();
            debug!("Byte mode: {} bytes in file", file_len);

            // If file exceeds chunk_size by itself, do forced splits
            if file_len >= chunk_size {
                debug!("File exceeds chunk size, splitting into multiple chunks");
                // Flush current buffer first
                if !buffer.is_empty() {
                    debug!("Flushing buffer before large file");
                    write_single_chunk(&buffer, chunk_idx, None, out_dir, is_stream)?;
                    buffer.clear();
                    used_size = 0;
                    chunk_idx += 1;
                }

                // Split large file into chunks
                let mut start = 0;
                let mut part = 0;
                while start < file_len {
                    let end = (start + chunk_size).min(file_len);
                    let chunk_data = &content.as_bytes()[start..end];
                    let chunk_str = format!(
                        "chunk {}\n>>>> {}:part {}\n{}\n",
                        chunk_idx,
                        rel_path,
                        part,
                        String::from_utf8_lossy(chunk_data)
                    );
                    debug!("Writing large file part {}", part);
                    write_single_chunk(&chunk_str, chunk_idx, Some(part), out_dir, is_stream)?;
                    chunk_idx += 1;
                    part += 1;
                    start = end;
                }
            } else {
                // Small enough to fit in one chunk
                let overhead = 10 + rel_path.len();
                let add_size = file_len + overhead;

                if used_size + add_size > chunk_size && !buffer.is_empty() {
                    debug!("Flushing buffer due to size limit");
                    write_single_chunk(&buffer, chunk_idx, None, out_dir, is_stream)?;
                    buffer.clear();
                    used_size = 0;
                    chunk_idx += 1;
                }

                debug!("Adding file to buffer");
                buffer.push_str(&format!("chunk {}\n>>>> {}\n", chunk_idx, rel_path));
                buffer.push_str(&content);
                buffer.push('\n');
                used_size += add_size;
            }
        }
    }

    // Flush final chunk if not empty
    if !buffer.is_empty() {
        debug!("Flushing final buffer");
        write_single_chunk(&buffer, chunk_idx, None, out_dir, is_stream)?;
    }

    debug!("Finished write_chunks");
    Ok(())
}

/// The main function that the tests call.
pub fn serialize_repo(repo_path: &Path, cfg: Option<&YekConfig>) -> Result<()> {
    let config = cfg.cloned().unwrap_or_default();

    // Process files in parallel
    let processed_files = process_files_parallel(repo_path, &config)?;

    // Convert to the format expected by write_chunks
    let entries: Vec<(String, String, i32)> = processed_files
        .into_iter()
        .map(|f| (f.rel_path, f.content, f.priority))
        .collect();

    // Write chunks
    write_chunks(&entries, &config, config.stream)?;

    Ok(())
}

/// Find yek.toml by walking up directories
pub fn find_config_file(start_path: &Path) -> Option<PathBuf> {
    let mut current = if start_path.is_absolute() {
        debug!(
            "Starting config search from absolute path: {}",
            start_path.display()
        );
        start_path.to_path_buf()
    } else {
        let path = std::env::current_dir().ok()?.join(start_path);
        debug!(
            "Starting config search from relative path: {}",
            path.display()
        );
        path
    };

    loop {
        let config_path = current.join("yek.toml");
        if config_path.exists() {
            return Some(config_path);
        }
        if !current.pop() {
            break;
        }
    }

    None
}

/// Merge config from a TOML file if present
pub fn load_config_file(path: &Path) -> Option<YekConfig> {
    debug!("Attempting to load config from: {}", path.display());
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to read config file: {}", e);
            return None;
        }
    };

    match toml::from_str::<YekConfig>(&content) {
        Ok(cfg) => {
            debug!("Successfully loaded config");
            // Validate the config
            let errors = validate_config(&cfg);
            if !errors.is_empty() {
                eprintln!("Invalid configuration in {}:", path.display());
                for error in errors {
                    eprintln!("  {}: {}", error.field, error.message);
                }
                None
            } else {
                Some(cfg)
            }
        }
        Err(e) => {
            eprintln!("Failed to parse config file: {}", e);
            None
        }
    }
}

/// Rank-based approach to compute how "recent" each file is (0=oldest, 1=newest).
/// Then scale it to a user-defined or default max boost.
#[allow(dead_code)]
fn compute_recentness_boost(
    commit_times: &HashMap<String, u64>,
    max_boost: i32,
) -> HashMap<String, i32> {
    if commit_times.is_empty() {
        return HashMap::new();
    }

    // Sort by ascending commit time => first is oldest
    let mut sorted: Vec<(&String, &u64)> = commit_times.iter().collect();
    sorted.sort_by_key(|(_, t)| **t);

    // oldest file => rank=0, newest => rank=1
    let last_index = sorted.len().saturating_sub(1) as f64;
    if last_index < 1.0 {
        // If there's only one file, or zero, no boosts make sense
        let mut single = HashMap::new();
        for file in commit_times.keys() {
            single.insert(file.clone(), 0);
        }
        return single;
    }

    let mut result = HashMap::new();
    for (i, (path, _time)) in sorted.iter().enumerate() {
        let rank = i as f64 / last_index; // 0.0..1.0 (older files get lower rank)
        let boost = (rank * max_boost as f64).round() as i32; // Newer files get higher boost
        result.insert((*path).clone(), boost);
    }
    result
}

#[cfg(target_family = "windows")]
#[allow(dead_code)]
fn is_effectively_absolute(path: &std::path::Path) -> bool {
    if path.is_absolute() {
        return true;
    }
    // Also treat a leading slash/backslash as absolute
    match path.to_str() {
        Some(s) => s.starts_with('/') || s.starts_with('\\'),
        None => false,
    }
}

#[cfg(not(target_family = "windows"))]
#[allow(dead_code)]
fn is_effectively_absolute(path: &std::path::Path) -> bool {
    path.is_absolute()
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

/// Parse size (for bytes or tokens) with optional K/KB, M/MB, G/GB suffix if not in token mode.
pub fn parse_size_input(input: &str, is_tokens: bool) -> Result<usize> {
    let s = input.trim();
    if is_tokens {
        // If user typed "128K", interpret as 128000 tokens
        if s.to_lowercase().ends_with('k') {
            let val = s[..s.len() - 1]
                .trim()
                .parse::<usize>()
                .map_err(|e| anyhow!("Invalid token size: {}", e))?;
            return Ok(val * 1000);
        }
        Ok(s.parse::<usize>()?)
    } else {
        // Byte-based suffix
        let s = s.to_uppercase();
        if s.ends_with("KB") {
            let val = s[..s.len() - 2].trim().parse::<usize>()?;
            return Ok(val * 1024);
        } else if s.ends_with("MB") {
            let val = s[..s.len() - 2].trim().parse::<usize>()?;
            return Ok(val * 1024 * 1024);
        } else if s.ends_with("GB") {
            let val = s[..s.len() - 2].trim().parse::<usize>()?;
            return Ok(val * 1024 * 1024 * 1024);
        } else if let Ok(val) = s.parse::<usize>() {
            return Ok(val);
        }
        Err(anyhow!("Invalid size string: {}", input))
    }
}
