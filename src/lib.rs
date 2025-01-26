use anyhow::{anyhow, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::{Command as SysCommand, Stdio};
use std::{fs, str};
use tracing::debug;

mod defaults;
pub mod model_manager;
mod parallel;

use defaults::BINARY_FILE_EXTENSIONS;
use parallel::process_files_parallel;

/// Convert a glob pattern to a regex pattern
fn glob_to_regex(pattern: &str) -> String {
    pattern
        .replace(".", "\\.")
        .replace("*", "[^/]*") // Match any character except /
        .replace("?", "[^/]") // Match any single character except /
        .replace("[!", "[^")
        .replace("{", "(")
        .replace("}", ")")
        .replace(",", "|")
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
    pub output_dir: Option<PathBuf>,
    #[serde(default)]
    pub stream: bool,
    #[serde(default)]
    pub token_mode: bool,
    #[serde(default)]
    pub tokenizer_model: Option<String>,
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
        // Merge other fields as needed, for example:
        self.ignore_patterns.extend(other.ignore_patterns.clone());
        self.priority_rules.extend(other.priority_rules.clone());
        self.binary_extensions
            .extend(other.binary_extensions.clone());
    }
}

/// Check if file is text by extension or scanning first chunk for null bytes.
pub fn is_text_file(path: &Path, user_binary_extensions: &[String]) -> io::Result<bool> {
    // Check user-provided binary extensions first, permitting no leading dot
    if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
        let ext_lower = ext.to_lowercase();
        if user_binary_extensions
            .iter()
            .any(|e| e.trim_start_matches('.') == ext_lower)
        {
            return Ok(false);
        }
    }

    // Check default binary extensions
    if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
        if BINARY_FILE_EXTENSIONS.contains(&ext.to_lowercase().as_str()) {
            return Ok(false);
        }
    }

    // If no extension or not in binary list, check content
    let mut file = fs::File::open(path)?;
    let mut buffer = [0; 512]; // Read a small chunk to check for null bytes
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
    pub message: String,
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

pub fn validate_config(config: &YekConfig) -> Vec<ConfigError> {
    let mut errors = Vec::new();

    // Validate priority rules
    for rule in &config.priority_rules {
        if rule.score < 0 || rule.score > 1000 {
            errors.push(ConfigError {
                message: format!("Priority score {} must be between 0 and 1000", rule.score),
            });
        }
        if rule.pattern.is_empty() {
            errors.push(ConfigError {
                message: "Priority rule must have a pattern".to_string(),
            });
        }
        // Validate regex pattern
        if let Err(e) = Regex::new(&rule.pattern) {
            errors.push(ConfigError {
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
                message: format!("Invalid pattern '{}': {}", pattern, e),
            });
        }
    }

    // Validate max_size
    if let Some(size) = config.max_size {
        if size == 0 {
            errors.push(ConfigError {
                message: "Max size cannot be 0".to_string(),
            });
        }
    }

    // Validate output directory if specified
    if let Some(dir) = &config.output_dir {
        let path = Path::new(dir);
        if path.exists() && !path.is_dir() {
            errors.push(ConfigError {
                message: format!(
                    "Output path '{}' exists but is not a directory",
                    dir.display()
                ),
            });
        }

        if let Err(e) = std::fs::create_dir_all(path) {
            errors.push(ConfigError {
                message: format!("Cannot create output directory '{}': {}", dir.display(), e),
            });
        }
    }

    errors
}

/// Returns a relative, normalized path string (forward slashes on all platforms).
pub fn normalize_path(path: &Path) -> String {
    if path.to_str() == Some(".") {
        return ".".to_string();
    }

    let path_str = path.to_string_lossy().replace('\\', "/");
    let stripped = path_str.strip_prefix("./").unwrap_or(&path_str);
    let trimmed = stripped.trim_start_matches('/').trim_end_matches('/');

    if trimmed.is_empty() {
        ".".to_string()
    } else {
        trimmed.to_string()
    }
}

pub fn normalize_path_with_root(path: &Path, base: &Path) -> String {
    let path = match path.strip_prefix(base) {
        Ok(p) => p,
        Err(_) => path,
    };
    normalize_path(path)
}

/// The main function that the tests call.
pub fn serialize_repo(repo_path: &Path, cfg: Option<&YekConfig>) -> Result<()> {
    let config = cfg.cloned().unwrap_or_default();
    let _is_stream = config.stream;

    // Process files in parallel

    let mut output = Vec::new();
    process_files_parallel(repo_path, &config, &mut output)?;
    let final_output = output.join("\n");
    if config.stream {
        // Write to stdout
        print!("{}", final_output);
    } else {
        // Determine output directory
        let output_dir = config
            .output_dir
            .as_deref()
            .unwrap_or_else(|| Path::new("."));

        // Create directory if it doesn't exist
        fs::create_dir_all(output_dir)?;

        // Write to output.txt in the output directory
        let output_path = output_dir.join("output.txt");
        fs::write(output_path, final_output)?;
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
pub fn load_config_file(path: impl AsRef<Path>) -> Option<YekConfig> {
    let path = path.as_ref();
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
                    eprintln!("  {}", error.message);
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

pub fn is_ignored(path: &str, patterns: &[String]) -> bool {
    patterns.iter().any(|p| {
        let pattern = if p.starts_with('^') || p.ends_with('$') {
            // If it's already a regex pattern, use it as is
            p.to_string()
        } else {
            // Convert glob pattern to regex, handling special cases
            let mut pattern = glob_to_regex(p);
            if !pattern.starts_with('^') {
                pattern = format!("^{}", pattern);
            }
            if !pattern.ends_with('$') {
                pattern = format!("{}$", pattern);
            }
            pattern
        };

        if let Ok(re) = Regex::new(&pattern) {
            re.is_match(path)
        } else {
            false
        }
    })
}
