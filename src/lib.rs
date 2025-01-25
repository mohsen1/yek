use anyhow::{anyhow, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::Command as SysCommand;
use std::{fs, str};
use tracing::debug;

mod defaults;
pub mod model_manager;
mod parallel;

use defaults::BINARY_FILE_EXTENSIONS;
use parallel::process_files_parallel;

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
    #[serde(default)]
    pub max_files: Option<usize>,
}

impl Default for YekConfig {
    fn default() -> Self {
        Self {
            stream: false,
            output_dir: None,
            priority_rules: vec![],
            binary_extensions: vec![
                "jpg".to_string(),
                "jpeg".to_string(),
                "png".to_string(),
                "gif".to_string(),
                "bin".to_string(),
                "zip".to_string(),
                "exe".to_string(),
                "dll".to_string(),
                "so".to_string(),
                "dylib".to_string(),
                "class".to_string(),
                "jar".to_string(),
                "pyc".to_string(),
                "pyo".to_string(),
                "pyd".to_string(),
            ],
            ignore_patterns: vec![],
            token_mode: false,
            tokenizer_model: None,
            max_size: None,
            max_files: None,
        }
    }
}

impl YekConfig {
    pub fn merge(&mut self, other: &YekConfig) {
        // Only override output_dir if present in other config
        if other.output_dir.is_some() {
            self.output_dir = other.output_dir.clone();
        }
        self.stream = other.stream;
        self.token_mode = other.token_mode;
        if other.max_size.is_some() {
            self.max_size = other.max_size;
        }
        if other.max_files.is_some() {
            self.max_files = other.max_files;
        }
        if other.tokenizer_model.is_some() {
            self.tokenizer_model = other.tokenizer_model.clone();
        }

        // Merge ignore patterns (append and deduplicate)
        let mut seen_ignores = HashSet::new();
        self.ignore_patterns
            .retain(|p| seen_ignores.insert(p.clone()));
        for pattern in &other.ignore_patterns {
            if seen_ignores.insert(pattern.clone()) {
                self.ignore_patterns.push(pattern.clone());
            }
        }

        // Merge binary extensions (normalize and deduplicate)
        let mut seen_exts = HashSet::new();
        let normalize_ext = |ext: &str| ext.trim_start_matches('.').to_lowercase();

        self.binary_extensions
            .retain(|ext| seen_exts.insert(normalize_ext(ext)));

        for ext in &other.binary_extensions {
            let normalized = normalize_ext(ext);
            if seen_exts.insert(normalized) {
                self.binary_extensions.push(ext.clone());
            }
        }

        // Merge priority rules (keep highest score per pattern)
        let mut rule_map: HashMap<String, i32> = self
            .priority_rules
            .drain(..)
            .map(|r| (r.pattern, r.score))
            .collect();

        for rule in &other.priority_rules {
            rule_map
                .entry(rule.pattern.clone())
                .and_modify(|s| *s = (*s).max(rule.score))
                .or_insert(rule.score);
        }

        self.priority_rules = rule_map
            .into_iter()
            .map(|(pattern, score)| PriorityRule { pattern, score })
            .collect();
    }
}

#[derive(Debug)]
pub struct ConfigError {
    pub message: String,
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

pub fn validate_config(config: &YekConfig) -> Vec<ConfigError> {
    let mut errors = Vec::new();

    if let Some(max_size) = config.max_size {
        if max_size == 0 {
            errors.push(ConfigError {
                message: "max_size cannot be zero".to_string(),
            });
        }
    }

    if let Some(max_files) = config.max_files {
        if max_files == 0 {
            errors.push(ConfigError {
                message: "max_files cannot be zero".to_string(),
            });
        }
    }

    errors
}

/// Check if file is text by extension or scanning first part for null bytes.
pub fn is_text_file(path: &Path, user_binary_extensions: &[String]) -> io::Result<bool> {
    let extension = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.trim_start_matches('.').to_string());

    // Check user-provided binary extensions
    if let Some(ext) = &extension {
        if user_binary_extensions
            .iter()
            .any(|e| e.trim_start_matches('.') == ext)
        {
            return Ok(false);
        }
    }

    // Check default binary extensions
    if let Some(ext) = &extension {
        if BINARY_FILE_EXTENSIONS.iter().any(|&e| e == ext) {
            return Ok(false);
        }
    }

    // If no extension or not in binary list, try to read first few bytes
    let mut file = fs::File::open(path)?;
    let mut buffer = [0; 1024];
    let n = file.read(&mut buffer)?;

    // Check for null bytes in the first chunk
    Ok(!buffer[..n].contains(&0))
}

/// Determine final priority of a file by scanning the priority list
/// in descending order of score.
pub fn get_file_priority(path: &str, rules: &[PriorityRule]) -> i32 {
    // Strip leading ./ if present
    let path = path.strip_prefix("./").unwrap_or(path);

    // Default priority of 0 for files without explicit rules
    let mut max_priority = 0;

    for rule in rules {
        if let Ok(re) = Regex::new(&glob_to_regex(&rule.pattern)) {
            if re.is_match(path) {
                max_priority = max_priority.max(rule.score);
            }
        }
    }
    max_priority
}

/// Get the commit time of the most recent change to each file.
/// Returns a map from file path (relative to the repo root) → last commit Unix time.
/// If Git or .git folder is missing, returns None instead of erroring.
pub fn get_recent_commit_times(repo_path: &Path) -> Option<HashMap<String, u64>> {
    // Check if .git directory exists
    if !repo_path.join(".git").exists() {
        return None;
    }

    // Initialize git repo if needed
    let status = SysCommand::new("git")
        .args(["-C", repo_path.to_str().unwrap(), "rev-parse", "--git-dir"])
        .output()
        .ok()?;

    if !status.status.success() {
        return None;
    }

    let mut result = HashMap::new();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_secs();

    // Get list of all files in the index (both staged and committed)
    let ls_output = SysCommand::new("git")
        .args([
            "-C",
            repo_path.to_str().unwrap(),
            "ls-files",
            "--cached",
            "--full-name",
        ])
        .output()
        .ok()?;

    if !ls_output.status.success() {
        return None;
    }

    let files: Vec<String> = String::from_utf8_lossy(&ls_output.stdout)
        .lines()
        .map(|s| s.to_string())
        .collect();

    // Get commit times for each file
    for file in files {
        let log_output = SysCommand::new("git")
            .args([
                "-C",
                repo_path.to_str().unwrap(),
                "log",
                "-1",
                "--format=%ct",
                "--",
                &file,
            ])
            .output()
            .ok()?;

        if log_output.status.success() {
            let timestamp = String::from_utf8_lossy(&log_output.stdout)
                .trim()
                .parse()
                .unwrap_or(now);
            result.insert(file, timestamp);
        } else {
            // If no commit history (newly staged file), use current time
            result.insert(file, now);
        }
    }

    if result.is_empty() {
        None
    } else {
        Some(result)
    }
}

pub const DEFAULT_PART_SIZE: usize = 10 * 1024 * 1024; // 10MB in README

/// The main function that the tests call.
pub fn serialize_repo(repo_path: &Path, cfg: Option<&YekConfig>) -> Result<()> {
    let config = cfg.cloned().unwrap_or_default();
    process_directory(repo_path, &config)
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
pub fn normalize_path(path: &Path) -> String {
    let path_str = path.to_string_lossy().replace('\\', "/");
    let stripped = if let Some(s) = path_str.strip_prefix("./") {
        s
    } else {
        &path_str
    };
    let trimmed = stripped.trim_start_matches('/').trim_end_matches('/');

    if trimmed.is_empty() {
        ".".to_string()
    } else {
        trimmed.to_string()
    }
}

/// Parse size input with mode-specific unit requirements:
/// - Token mode (--tokens): K/M units (e.g., "128K" = 128,000 tokens)
/// - Bytes mode (default): KB/MB/GB units (e.g., "128KB" = 131,072 bytes)
pub fn parse_size_input(input: &str, is_tokens: bool) -> Result<usize> {
    let input = input.trim().to_uppercase();

    if is_tokens {
        // Token mode: K/M without B suffix
        let re = Regex::new(r"^(\d+)([KM])?$").unwrap();
        if let Some(caps) = re.captures(&input) {
            let num: usize = caps.get(1).unwrap().as_str().parse()?;
            let unit = caps.get(2).map(|m| m.as_str()).unwrap_or("");

            let multiplier = match unit {
                "" => 1,
                "K" => 1000,
                "M" => 1000 * 1000,
                _ => return Err(anyhow!("Invalid token unit: {}. Use K or M", unit)),
            };

            Ok(num * multiplier)
        } else {
            Err(anyhow!(
                "Invalid token format: {}. Examples: 100K, 1M",
                input
            ))
        }
    } else {
        // Bytes mode: requires KB/MB/GB suffix
        let re = Regex::new(r"^(\d+)(KB|MB|GB|TB|B)?$").unwrap();
        if let Some(caps) = re.captures(&input) {
            let num: usize = caps.get(1).unwrap().as_str().parse()?;
            let unit = caps.get(2).map(|m| m.as_str()).unwrap_or("");

            let multiplier = match unit {
                "B" | "" => 1,
                "KB" => 1024,
                "MB" => 1024 * 1024,
                "GB" => 1024 * 1024 * 1024,
                "TB" => 1024 * 1024 * 1024 * 1024,
                _ => {
                    return Err(anyhow!(
                        "Invalid byte unit: {}. Use KB, MB, GB, or TB",
                        unit
                    ))
                }
            };

            Ok(num * multiplier)
        } else {
            Err(anyhow!(
                "Invalid byte format: {}. Examples: 100KB, 1MB",
                input
            ))
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

fn is_ignored(path: &str, patterns: &[String]) -> bool {
    for pattern in patterns {
        let re = glob_to_regex(pattern);
        if Regex::new(&re).unwrap().is_match(path) {
            return true;
        }
    }
    false
}

pub fn normalize_path_with_root(path: &Path, root: &Path) -> String {
    let stripped_path = match path.strip_prefix(root) {
        Ok(p) => p,
        Err(_) => path.file_name().map_or_else(|| path, |f| Path::new(f)),
    };
    normalize_path(stripped_path)
}

pub fn process_directory(path: &Path, config: &YekConfig) -> Result<()> {
    let mut config = config.clone();
    if config.output_dir.is_none() {
        // use temp dir if no output dir is specified
        config.output_dir = Some(std::env::temp_dir().join("yek-output"));
    }

    let mut output_chunks = Vec::new();
    process_files_parallel(path, &config, &mut output_chunks)?;

    let output = output_chunks.join("\n");
    let is_stream = config.stream;

    // Ensure output directory exists even if no content
    if let Some(out_dir) = &config.output_dir {
        fs::create_dir_all(out_dir)?;
    }

    if is_stream {
        print!("{}", output);
        Ok(())
    } else {
        // Write output even if empty to indicate completion
        let output_path = config.output_dir.unwrap().join("output.txt");
        fs::write(&output_path, &output)?;
        println!("{}", output_path.display());
        Ok(())
    }
}
