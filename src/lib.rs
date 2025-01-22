use anyhow::{anyhow, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::collections::HashSet;
use std::fs;
use std::io;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command as SysCommand, Stdio};
use tracing::debug;

mod defaults;
pub mod model_manager;
mod parallel;

use defaults::{BINARY_FILE_EXTENSIONS, TEXT_FILE_EXTENSIONS};
use parallel::process_files_parallel;

/// Write output to either a single file or stdout
fn write_output(content: &str, out_dir: Option<&Path>, is_stream: bool) -> Result<()> {
    if is_stream {
        print!("{}", content);
        Ok(())
    } else {
        let out_dir = out_dir.expect("output dir required for file mode");
        fs::write(out_dir.join("output.txt"), content)?;
        Ok(())
    }
}

/// Safely truncate a string to specified byte length
fn truncate_bytes(s: &str, max_bytes: usize) -> String {
    if s.len() <= max_bytes {
        return s.to_string();
    }
    let mut end = max_bytes;
    while !s.is_char_boundary(end) {
        end -= 1;
    }
    s[..end].to_string()
}

/// Convert a glob pattern to a regex pattern
fn glob_to_regex(pattern: &str) -> String {
    let adjusted_pattern = if pattern.ends_with('/') {
        format!("{}**", pattern)
    } else {
        pattern.to_string()
    };
    let mut regex = String::with_capacity(adjusted_pattern.len() * 2);
    regex.push('^');
    let mut chars = adjusted_pattern.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '*' => {
                if chars.peek() == Some(&'*') {
                    chars.next(); // consume second *
                                  // Handle /**/ pattern to match zero or more directories
                    if chars.peek() == Some(&'/') {
                        chars.next(); // consume /
                        regex.push_str("(?:.*/)?");
                    } else {
                        regex.push_str(".*");
                    }
                } else {
                    // Single * matches non-slash characters
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
    regex.push('$');
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

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    pub token_mode: bool,
    #[serde(default)]
    pub tokenizer_model: Option<String>,
    #[serde(default)]
    pub stream: bool,
    #[serde(default)]
    pub output_dir: Option<PathBuf>,
}

impl Default for YekConfig {
    fn default() -> Self {
        Self {
            stream: false,
            output_dir: None,
            priority_rules: vec![],
            binary_extensions: vec![
                "jpg".into(),
                "jpeg".into(),
                "png".into(),
                "gif".into(),
                "bin".into(),
                "zip".into(),
                "exe".into(),
                "dll".into(),
                "so".into(),
                "dylib".into(),
                "class".into(),
                "jar".into(),
                "pyc".into(),
                "pyo".into(),
                "pyd".into(),
            ],
            ignore_patterns: vec![],
            token_mode: false,
            tokenizer_model: None,
            max_size: None,
        }
    }
}

pub fn validate_config(config: &YekConfig) -> Vec<ConfigError> {
    let mut errors = Vec::new();

    // Validate tokenizer model if specified or in token mode
    if config.token_mode {
        // In token mode, we always have a model (default or specified)
        let model = config.tokenizer_model.as_deref().unwrap_or("openai");
        debug!("Token mode enabled with model: {}", model);
        if !model_manager::SUPPORTED_MODEL_FAMILIES.contains(&model) {
            errors.push(ConfigError {
                field: "tokenizer_model".to_string(),
                message: format!(
                    "Unsupported model '{}'. Supported models: {}",
                    model,
                    model_manager::SUPPORTED_MODEL_FAMILIES.join(", ")
                ),
            });
        }
    }

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

/// Check if file is text by extension or scanning first part for null bytes.
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
    // Strip leading ./ if present
    let path = path.strip_prefix("./").unwrap_or(path);
    rules
        .iter()
        .filter_map(|rule| {
            let regex_pattern = glob_to_regex(&rule.pattern);
            let re = match Regex::new(&regex_pattern) {
                Ok(re) => re,
                Err(e) => {
                    tracing::warn!("Invalid regex pattern {}: {}", rule.pattern, e);
                    return None;
                }
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

    // Get all files and their timestamps using git log
    let output = SysCommand::new("git")
        .args([
            "-C",
            repo_path.to_str()?,
            "log",
            "--format=%ct",
            "--name-only",
            "--no-merges",
            "--no-renames",
            "--",
            ".",
        ])
        .stderr(Stdio::null())
        .output()
        .ok()?;

    if !output.status.success() {
        debug!("Git log command failed, skipping Git-based prioritization");
        return None;
    }

    let mut git_times: HashMap<String, u64> = HashMap::new();
    let mut current_timestamp = 0_u64;

    // Process output line by line
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
            // Only update the timestamp if it's more recent than what we have
            git_times
                .entry(line.to_string())
                .and_modify(|e| *e = (*e).max(current_timestamp))
                .or_insert(current_timestamp);
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

pub const DEFAULT_PART_SIZE: usize = 10 * 1024 * 1024; // 10MB in README

/// The main function that the tests call.
pub fn serialize_repo(repo_path: &Path, cfg: Option<&YekConfig>) -> Result<()> {
    let config = cfg.cloned().unwrap_or_default();

    // Log tokenizer configuration
    if config.token_mode {
        debug!(
            "Token mode enabled with model: {:?}",
            config.tokenizer_model.as_deref().unwrap_or("openai")
        );
    }

    // Validate config before processing
    let errors = validate_config(&config);
    if !errors.is_empty() {
        for error in errors {
            eprintln!("Error in {}: {}", error.field, error.message);
        }
        return Err(anyhow!("Invalid configuration"));
    }

    // Create output directory early even if empty
    if let Some(output_dir) = &config.output_dir {
        if !config.stream {
            // Create directory without .keep file to avoid unnecessary I/O
            std::fs::create_dir_all(output_dir).map_err(|e: std::io::Error| {
                anyhow!(
                    "Failed to create output directory '{}': {}",
                    output_dir.display(),
                    e
                )
            })?;
        }
    }

    // Process files in parallel
    let processed_files = process_files_parallel(repo_path, &config)?;

    // Convert ProcessedFile to the expected tuple format
    if processed_files.is_empty() {
        return Ok(()); // No files processed, nothing to write
    }

    let mut entries: Vec<(String, String, i32)> = processed_files
        .into_iter()
        .map(|f| (f.rel_path, f.content, f.priority))
        .collect();

    // Sort by descending priority (higher priority first)
    entries.sort_by(|a, b| b.2.cmp(&a.2));

    // Prepare content chunks
    let max_size = config.max_size.unwrap_or(usize::MAX);
    let mut chunks = Vec::new();
    let mut current_chunk = String::new();
    let mut current_size = 0;

    for (rel_path, file_content, _) in entries {
        let model = config.tokenizer_model.as_deref().unwrap_or("openai");
        let entry_header = format!(">>>> {}\n", rel_path);
        let entry = format!("{}{}\n", entry_header, file_content);
        let entry_size = if config.token_mode {
            model_manager::count_tokens(&entry, model).unwrap_or_else(|e| {
                tracing::warn!("Token count failed for {}: {}", rel_path, e);
                entry.len()
            })
        } else {
            entry.len()
        };

        // Check if entry exceeds max_size on its own
        if entry_size > max_size {
            let mut start = 0;
            let mut part = 0;

            while start < file_content.len() {
                let part_header = format!(">>>> {} (part {})\n", rel_path, part);
                let header_size = if config.token_mode {
                    model_manager::count_tokens(&part_header, model).unwrap_or_else(|e| {
                        tracing::warn!("Header token count failed: {}", e);
                        part_header.len()
                    })
                } else {
                    part_header.len()
                };

                let available_size = max_size.saturating_sub(header_size);
                if available_size <= 0 {
                    break;
                }

                let end = if config.token_mode {
                    // Tokenize the remaining content
                    let tokens = model_manager::tokenize(&file_content[start..], model)
                        .unwrap_or_else(|e| {
                            tracing::warn!("Tokenization failed: {}", e);
                            vec![]
                        });
                    let mut token_count = 0;
                    let mut char_pos = start;

                    for token in tokens.iter() {
                        let token_text =
                            model_manager::decode_tokens(&[*token], model).unwrap_or_default();
                        token_count += 1;
                        char_pos += token_text.len();

                        if token_count > available_size {
                            break;
                        }
                    }

                    (start + char_pos).min(file_content.len())
                } else {
                    // Byte mode handling
                    start + available_size.min(file_content.len() - start)
                };

                let part_content = &file_content[start..end];
                let full_part = format!("{}{}\n", part_header, part_content);
                chunks.push(full_part);

                start = end;
                part += 1;
            }
            continue;
        }

        // For non-split files, check if adding entry would exceed max_size
        let header_size = if config.token_mode {
            model_manager::count_tokens(&entry_header, model).unwrap_or_else(|e| {
                tracing::warn!("Header token count failed: {}", e);
                entry_header.len()
            })
        } else {
            entry_header.len()
        };

        let content_size = if config.token_mode {
            model_manager::count_tokens(&file_content, model).unwrap_or_else(|_| file_content.len())
        } else {
            file_content.len()
        };

        let total_size = header_size + content_size;

        if current_size + total_size > max_size && !current_chunk.is_empty() {
            chunks.push(current_chunk);
            current_chunk = String::new();
            current_size = 0;
        }

        current_chunk.push_str(&format!("{}{}\n", entry_header, file_content));
        current_size += total_size;
    }

    // Add remaining content
    if !current_chunk.is_empty() {
        chunks.push(current_chunk);
    }

    // Handle empty repo case
    if chunks.is_empty() {
        return Ok(());
    }

    // Write each chunk to a separate file or combine for streaming
    if config.stream {
        write_output(&chunks.join(""), config.output_dir.as_deref(), true)?;
    } else {
        let out_dir = config
            .output_dir
            .as_deref()
            .map(|d| std::fs::create_dir_all(d).map(|_| d))
            .transpose()?
            .expect("output dir required for file mode");

        if chunks.len() == 1 {
            fs::write(out_dir.join("output.txt"), &chunks[0])?;
        } else {
            for (i, chunk) in chunks.iter().enumerate() {
                fs::write(out_dir.join(format!("part-{}.txt", i)), chunk)?;
            }
            debug!("Wrote {} parts", chunks.len());
        }
    }

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
            Some(cfg)
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
    // Use the original base path without canonicalization
    let base: &Path = base.as_ref();
    let path: &Path = path.as_ref();

    // Attempt to get relative path directly without canonicalization
    match path.strip_prefix(base) {
        Ok(rel_path) => {
            let path_str = rel_path.to_string_lossy().replace('\\', "/");
            path_str.to_string()
        }
        Err(_) => path.to_string_lossy().replace('\\', "/"),
    }
}

/// Parse size (for bytes or tokens) with optional K/KB, M/MB, G/GB suffix if not in token mode.
pub fn parse_size_input(input: &str, is_tokens: bool) -> Result<usize> {
    let s = input.trim();
    if s.is_empty() {
        return Err(anyhow!("Size cannot be empty"));
    }

    if is_tokens {
        // If user typed "128K", interpret as 128000 tokens
        let s = s.to_lowercase();
        if s.ends_with('k') {
            let val = s[..s.len() - 1]
                .trim()
                .parse::<usize>()
                .map_err(|_| anyhow!("Invalid token size: {}", s))?;
            Ok(val * 1000)
        } else if s.ends_with('m') {
            let val = s[..s.len() - 1]
                .trim()
                .parse::<usize>()
                .map_err(|_| anyhow!("Invalid token size: {}", s))?;
            return Ok(val * 1000 * 1000);
        } else {
            // Plain number
            let val = s
                .parse::<usize>()
                .map_err(|_| anyhow!("Invalid token size: {}", s))?;
            return Ok(val);
        }
    } else {
        // Byte-based suffix
        let s = s.to_uppercase();
        if s.ends_with("KB") {
            let val = s[..s.len() - 2]
                .trim()
                .parse::<usize>()
                .map_err(|_| anyhow!("Invalid size: {}", input))?;
            Ok(val * 1024)
        } else if s.ends_with("MB") {
            let val = s[..s.len() - 2]
                .trim()
                .parse::<usize>()
                .map_err(|_| anyhow!("Invalid size: {}", input))?;
            Ok(val * 1024 * 1024)
        } else if s.ends_with("GB") {
            let val = s[..s.len() - 2]
                .trim()
                .parse::<usize>()
                .map_err(|_| anyhow!("Invalid size: {}", input))?;
            Ok(val * 1024 * 1024 * 1024)
        } else {
            // Plain bytes
            s.parse::<usize>()
                .map_err(|_| anyhow!("Invalid size: {}", input))
        }
    }
}

pub fn merge_config(dest: &mut YekConfig, other: &YekConfig) {
    // Merge ignore patterns, removing duplicates
    let mut seen_patterns = HashSet::new();
    let mut merged_patterns = Vec::new();

    // Add patterns from dest first (base config)
    for pattern in &dest.ignore_patterns {
        if seen_patterns.insert(pattern.clone()) {
            merged_patterns.push(pattern.clone());
        }
    }

    // Add patterns from other (overlay config)
    for pattern in &other.ignore_patterns {
        if seen_patterns.insert(pattern.clone()) {
            merged_patterns.push(pattern.clone());
        }
    }
    dest.ignore_patterns = merged_patterns;

    // Merge binary extensions, removing duplicates
    let mut seen_extensions = HashSet::new();
    let mut merged_extensions = Vec::new();

    // Add extensions from dest first
    for ext in &dest.binary_extensions {
        let normalized = ext.trim_start_matches('.').to_lowercase();
        if seen_extensions.insert(normalized.clone()) {
            merged_extensions.push(normalized);
        }
    }

    // Add extensions from other
    for ext in &other.binary_extensions {
        let normalized = ext.trim_start_matches('.').to_lowercase();
        if seen_extensions.insert(normalized.clone()) {
            merged_extensions.push(normalized);
        }
    }
    dest.binary_extensions = merged_extensions;

    // Merge priority rules, keeping the highest score for each pattern
    let mut priority_map = HashMap::new();

    // Process rules from dest first
    for rule in &dest.priority_rules {
        priority_map
            .entry(rule.pattern.clone())
            .and_modify(|e: &mut i32| *e = (*e).max(rule.score))
            .or_insert(rule.score);
    }

    // Process rules from other, keeping higher scores
    for rule in &other.priority_rules {
        priority_map
            .entry(rule.pattern.clone())
            .and_modify(|e| *e = (*e).max(rule.score))
            .or_insert(rule.score);
    }

    // Convert back to Vec<PriorityRule>
    dest.priority_rules = priority_map
        .into_iter()
        .map(|(pattern, score)| PriorityRule { pattern, score })
        .collect();

    // Take other config values if they're set
    if other.max_size.is_some() {
        dest.max_size = other.max_size;
    }
    if other.token_mode {
        dest.token_mode = true;
    }
    if dest.token_mode {
        dest.tokenizer_model
            .get_or_insert_with(|| "openai".to_string());
    }
    if other.tokenizer_model.is_some() {
        dest.tokenizer_model = other.tokenizer_model.clone();
    }
    if other.stream {
        dest.stream = true;
    }
    if other.output_dir.is_some() {
        dest.output_dir = other.output_dir.clone();
    }
}
