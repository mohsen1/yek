use anyhow::Result;
use ignore::gitignore::GitignoreBuilder;
use regex::Regex;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command as SysCommand, Stdio};
use tracing::{debug, info};
use walkdir::WalkDir;
mod parallel;
use parallel::process_files_parallel;

/// Helper macro to write debug statements both to standard debug log and to debug file if set.
#[macro_export]
macro_rules! debug_file {
    ($($arg:tt)*) => {{
        let msg = format!($($arg)*);
        debug!("{}", msg);
        write_debug_to_file(&msg);
    }};
}

/// When the test uses `--debug` plus sets `YEK_DEBUG_OUTPUT`, we append key messages to that file.
fn write_debug_to_file(msg: &str) {
    if let Ok(path) = std::env::var("YEK_DEBUG_OUTPUT") {
        // Create parent directory if it doesn't exist
        if let Some(parent) = Path::new(&path).parent() {
            let _ = fs::create_dir_all(parent);
        }
        // Append the debug text to the file
        if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(&path) {
            let _ = writeln!(f, "{}", msg);
        }
    }
}

/// We provide an optional config that can add or override ignore patterns
/// and priority rules. All fields are optional and merged with defaults.
#[derive(Debug, Deserialize, Clone)]
pub struct YekConfig {
    #[serde(default)]
    pub ignore_patterns: IgnoreConfig,
    #[serde(default)]
    pub priority_rules: Vec<PriorityRule>,
    #[serde(default)]
    pub binary_extensions: Vec<String>,
    #[serde(default)]
    pub output_dir: Option<String>,
    #[serde(default)]
    pub git_boost_max: Option<i32>,
    #[serde(default)]
    pub channel_capacity: Option<usize>,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct IgnoreConfig {
    #[serde(default)]
    pub patterns: Vec<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct PriorityRule {
    pub score: i32,
    pub patterns: Vec<String>,
}

/// BINARY file checks by extension
const BINARY_FILE_EXTENSIONS: &[&str] = &[
    ".jpg", ".pdf", ".mid", ".blend", ".p12", ".rco", ".tgz", ".jpeg", ".mp4", ".midi", ".crt",
    ".p7b", ".ovl", ".bz2", ".png", ".webm", ".aac", ".key", ".gbr", ".mo", ".xz", ".gif", ".mov",
    ".flac", ".pem", ".pcb", ".nib", ".dat", ".ico", ".mp3", ".bmp", ".der", ".icns", ".xap",
    ".lib", ".webp", ".wav", ".psd", ".png2", ".xdf", ".psf", ".jar", ".ttf", ".exe", ".ai",
    ".jp2", ".zip", ".pak", ".vhd", ".woff", ".dll", ".eps", ".swc", ".rar", ".img3", ".gho",
    ".woff2", ".bin", ".raw", ".mso", ".7z", ".img4", ".efi", ".eot", ".iso", ".tif", ".class",
    ".gz", ".msi", ".ocx", ".sys", ".img", ".tiff", ".apk", ".tar", ".cab", ".scr", ".so", ".dmg",
    ".3ds", ".com", ".elf", ".o", ".max", ".obj", ".drv", ".rom", ".a", ".vhdx", ".fbx", ".bpl",
    ".cpl",
];

/// We'll define a minimal default config. The user can override parts of it from a TOML file.
impl Default for YekConfig {
    fn default() -> Self {
        YekConfig {
            ignore_patterns: IgnoreConfig::default(),
            priority_rules: vec![
                // Default fallback - everything has same priority
                PriorityRule {
                    score: 1,
                    patterns: vec![".*".to_string()],
                },
            ],
            binary_extensions: Vec::new(), // User extensions only, we'll combine with BINARY_FILE_EXTENSIONS
            output_dir: None,
            git_boost_max: None,
            channel_capacity: None,
        }
    }
}

/// Internal struct that, after merging, holds the final list of ignore patterns and priorities.
struct FinalConfig {
    ignore_patterns: Vec<Regex>,
    priority_list: Vec<PriorityPattern>,
}

#[derive(Clone)]
pub struct PriorityPattern {
    pub score: i32,
    pub patterns: Vec<Regex>,
}

/// Default sets of priority patterns
fn default_priority_list() -> Vec<PriorityPattern> {
    Vec::new() // Return empty list - no default priorities
}

/// Default sets of ignore patterns (separate from .gitignore)
fn default_ignore_patterns() -> Vec<Regex> {
    let raw = vec![
        r"^LICENSE$",
        r"^\.git/",
        r"^\.next/",
        r"^node_modules/",
        r"^vendor/",
        r"^dist/",
        r"^build/",
        r"^out/",
        r"^target/",
        r"^bin/",
        r"^obj/",
        r"^\.idea/",
        r"^\.vscode/",
        r"^\.vs/",
        r"^\.settings/",
        r"^\.gradle/",
        r"^\.mvn/",
        r"^\.pytest_cache/",
        r"^__pycache__/",
        r"^\.sass-cache/",
        r"^\.vercel/",
        r"^\.turbo/",
        r"^coverage/",
        r"^test-results/",
        r"\.gitignore",
        r"pnpm-lock\.yaml",
        r"yek\.toml",
        r"package-lock\.json",
        r"yarn\.lock",
        r"Cargo\.lock",
        r"Gemfile\.lock",
        r"composer\.lock",
        r"mix\.lock",
        r"poetry\.lock",
        r"Pipfile\.lock",
        r"packages\.lock\.json",
        r"paket\.lock",
        r"\.pyc$",
        r"\.pyo$",
        r"\.pyd$",
        r"\.class$",
        r"\.o$",
        r"\.obj$",
        r"\.dll$",
        r"\.exe$",
        r"\.so$",
        r"\.dylib$",
        r"\.log$",
        r"\.tmp$",
        r"\.temp$",
        r"\.swp$",
        r"\.swo$",
        r"\.DS_Store$",
        r"Thumbs\.db$",
        r"\.env(\..+)?$",
        r"\.bak$",
        r"~$",
    ];
    raw.into_iter()
        .map(|pat| Regex::new(pat).unwrap())
        .collect()
}

/// Merge default + config
fn build_final_config(cfg: Option<YekConfig>) -> FinalConfig {
    let mut merged_ignore = default_ignore_patterns();
    let mut merged_priority = default_priority_list();

    if let Some(user_cfg) = cfg {
        // Extend ignore
        for user_pat in user_cfg.ignore_patterns.patterns {
            if let Ok(reg) = Regex::new(&user_pat) {
                merged_ignore.push(reg);
            }
        }
        // Add user priority rules without clearing defaults
        for user_rule in user_cfg.priority_rules {
            if user_rule.patterns.is_empty() {
                continue;
            }
            let mut existing_idx: Option<usize> = None;
            for (i, p) in merged_priority.iter().enumerate() {
                if p.score == user_rule.score {
                    existing_idx = Some(i);
                    break;
                }
            }
            let new_regexes: Vec<Regex> = user_rule
                .patterns
                .iter()
                .filter_map(|pat| Regex::new(pat).ok())
                .collect();
            if let Some(idx) = existing_idx {
                let mut cloned = merged_priority[idx].clone();
                cloned.patterns.extend(new_regexes);
                merged_priority[idx] = cloned;
            } else {
                merged_priority.push(PriorityPattern {
                    score: user_rule.score,
                    patterns: new_regexes,
                });
            }
        }
        // Sort priority rules in ascending order so higher scores come last
        merged_priority.sort_by(|a, b| a.score.cmp(&b.score));
    }

    FinalConfig {
        ignore_patterns: merged_ignore,
        priority_list: merged_priority,
    }
}

/// Check if file is text by extension or scanning first chunk for null bytes.
pub fn is_text_file(file_path: &Path, user_binary_extensions: &[String]) -> bool {
    debug!("Checking if file is text: {}", file_path.display());

    // First check extension - fast path
    if let Some(ext) = file_path.extension().and_then(|s| s.to_str()) {
        let dot_ext = format!(".{}", ext.to_lowercase());
        if BINARY_FILE_EXTENSIONS.contains(&dot_ext.as_str())
            || user_binary_extensions.contains(&dot_ext)
        {
            debug!(
                "File {} identified as binary by extension",
                file_path.display()
            );
            return false;
        }

        // Known text extensions - skip content check
        if matches!(
            dot_ext.as_str(),
            ".txt"
                | ".md"
                | ".rs"
                | ".toml"
                | ".yml"
                | ".yaml"
                | ".json"
                | ".js"
                | ".ts"
                | ".html"
                | ".css"
                | ".sh"
                | ".py"
                | ".rb"
                | ".pl"
                | ".php"
                | ".java"
                | ".c"
                | ".cpp"
                | ".h"
                | ".hpp"
                | ".go"
                | ".swift"
                | ".kt"
                | ".scala"
                | ".r"
                | ".m"
                | ".sql"
                | ".xml"
                | ".ini"
                | ".conf"
                | ".cfg"
                | ".properties"
        ) {
            return true;
        }
    }

    // Only do content check for unknown extensions
    let mut f = match File::open(file_path) {
        Ok(f) => f,
        Err(e) => {
            debug!("Failed to open file {}: {}", file_path.display(), e);
            return false;
        }
    };
    let mut buffer = [0u8; 4096];
    let read_bytes = match f.read(&mut buffer) {
        Ok(n) => n,
        Err(e) => {
            debug!("Failed to read file {}: {}", file_path.display(), e);
            return false;
        }
    };
    for &b in &buffer[..read_bytes] {
        if b == 0 {
            debug!(
                "File {} identified as binary by content",
                file_path.display()
            );
            return false;
        }
    }
    debug!("File {} identified as text", file_path.display());
    true
}

/// Naive token counting or raw byte length
pub fn count_size(text: &str, count_tokens: bool) -> usize {
    if count_tokens {
        text.split_whitespace().count()
    } else {
        text.len()
    }
}

pub fn format_size(size: usize, is_tokens: bool) -> String {
    if is_tokens {
        format!("{} tokens", size)
    } else {
        let mut sizef = size as f64;
        let units = ["B", "KB", "MB", "GB"];
        let mut index = 0;
        while sizef >= 1024.0 && index < units.len() - 1 {
            sizef /= 1024.0;
            index += 1;
        }
        format!("{:.1} {}", sizef, units[index])
    }
}

/// Write chunk to file or stdout
fn write_chunk(
    files: &[(String, String)],
    index: usize,
    output_dir: Option<&Path>,
    stream: bool,
    count_tokens: bool,
) -> Result<usize> {
    let mut chunk_data = String::new();
    for (path, content) in files {
        chunk_data.push_str(">>>> ");
        #[cfg(windows)]
        chunk_data.push_str(&path.replace('\\', "/"));
        #[cfg(not(windows))]
        chunk_data.push_str(path);
        chunk_data.push('\n');
        chunk_data.push_str(content);
        chunk_data.push_str("\n\n");
    }
    let size = count_size(&chunk_data, count_tokens);

    if stream {
        let stdout = io::stdout();
        let mut handle = stdout.lock();
        handle.write_all(chunk_data.as_bytes())?;
        handle.flush()?;
    } else if let Some(dir) = output_dir {
        let chunk_path = dir.join(format!("chunk-{}.txt", index));
        let f = File::create(&chunk_path)?;
        let mut w = BufWriter::new(f);
        w.write_all(chunk_data.as_bytes())?;
        w.flush()?;

        info!(
            "Written chunk {} with {} files ({}).",
            index,
            files.len(),
            format_size(size, count_tokens)
        );
    }

    Ok(size)
}

/// Determine final priority of a file by scanning the priority list
/// in descending order of score.
pub fn get_file_priority(
    rel_str: &str,
    _ignore_pats: &[Regex],
    prio_list: &[PriorityPattern],
) -> i32 {
    // Loop from highest score → lowest
    for prio in prio_list.iter().rev() {
        for pat in &prio.patterns {
            if pat.is_match(rel_str) {
                return prio.score;
            }
        }
    }
    0 // fallback if nothing matches - lower than any user-defined priority
}

/// Get the commit time of the most recent change to each file.
/// Returns a map from file path (relative to the repo root) → last commit Unix time.
/// If Git or .git folder is missing, returns None instead of erroring.
pub fn get_recent_commit_times(repo_root: &Path) -> Option<HashMap<String, u64>> {
    // Confirm there's a .git folder
    if !repo_root.join(".git").exists() {
        debug!("No .git directory found, skipping Git-based prioritization");
        return None;
    }

    let output = SysCommand::new("git")
        .args([
            "log",
            "--pretty=format:%ct",
            "--name-only",
            "--no-merges",
            "--relative",
        ])
        .current_dir(repo_root)
        .stderr(Stdio::null())
        .output()
        .ok()?;

    if !output.status.success() {
        debug!("Git command failed, skipping Git-based prioritization");
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut map: HashMap<String, u64> = HashMap::new();
    let mut current_timestamp = 0_u64;

    for line in stdout.lines() {
        if line.is_empty() {
            continue;
        }
        if let Ok(ts) = line.parse::<u64>() {
            current_timestamp = ts;
        } else if !line.contains('\0') {
            // Skip any binary filenames
            map.insert(line.to_string(), current_timestamp);
        }
    }

    Some(map)
}

#[derive(Debug)]
struct FileEntry {
    path: PathBuf,
    priority: i32,
}

/// Validate the config object, returning any errors found
#[derive(Debug)]
pub struct ConfigError {
    pub field: String,
    pub message: String,
}

pub fn validate_config(config: &YekConfig) -> Vec<ConfigError> {
    let mut errors = Vec::new();

    // Validate ignore patterns
    for pattern in &config.ignore_patterns.patterns {
        if let Err(e) = Regex::new(pattern) {
            errors.push(ConfigError {
                field: "ignore_patterns".to_string(),
                message: format!("Invalid regex pattern '{}': {}", pattern, e),
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
        for pattern in &rule.patterns {
            if let Err(e) = Regex::new(pattern) {
                errors.push(ConfigError {
                    field: "priority_rules".to_string(),
                    message: format!("Invalid regex pattern '{}': {}", pattern, e),
                });
            }
        }
    }

    // Validate output directory if specified
    if let Some(dir) = &config.output_dir {
        let path = Path::new(dir);
        if path.exists() && !path.is_dir() {
            errors.push(ConfigError {
                field: "output_dir".to_string(),
                message: format!("Output path '{}' exists but is not a directory", dir),
            });
        } else if !path.exists() {
            if let Err(e) = std::fs::create_dir_all(path) {
                errors.push(ConfigError {
                    field: "output_dir".to_string(),
                    message: format!("Cannot create output directory '{}': {}", dir, e),
                });
            } else {
                let _ = std::fs::remove_dir(path);
            }
        }
    }

    errors
}

/// Core function to serialize files
pub fn serialize_repo(
    max_size: usize,
    base_path: Option<&Path>,
    stream: bool,
    count_tokens: bool,
    config: Option<YekConfig>,
    output_dir: Option<&Path>,
    _max_files: Option<usize>,
) -> Result<Option<PathBuf>> {
    let base_path = base_path.unwrap_or_else(|| Path::new("."));
    let final_config = build_final_config(config.clone());

    // Get git commit times if available
    let commit_times = get_recent_commit_times(base_path);

    // If we have commit times, compute a "recentness" map
    // that ranks all files from oldest to newest.
    let recentness_boost = if let Some(ref times) = commit_times {
        let max_boost = config.as_ref().and_then(|c| c.git_boost_max).unwrap_or(100);
        Some(compute_recentness_boost(times, max_boost))
    } else {
        None
    };

    // Build gitignore matcher
    let mut builder = GitignoreBuilder::new(base_path);
    let gitignore_path = base_path.join(".gitignore");
    if gitignore_path.exists() {
        builder.add(&gitignore_path);
    }
    let gitignore = builder
        .build()
        .unwrap_or_else(|_| GitignoreBuilder::new(base_path).build().unwrap());

    // Create output directory if needed
    let output_dir = if !stream {
        if let Some(dir) = output_dir {
            fs::create_dir_all(dir)?;
            Some(dir.to_path_buf())
        } else if let Some(cfg) = &config {
            if let Some(dir) = &cfg.output_dir {
                let path = Path::new(dir);
                fs::create_dir_all(path)?;
                Some(path.to_path_buf())
            } else {
                let dir = std::env::temp_dir().join("yek");
                fs::create_dir_all(&dir)?;
                Some(dir)
            }
        } else {
            let dir = std::env::temp_dir().join("yek");
            fs::create_dir_all(&dir)?;
            Some(dir)
        }
    } else {
        None
    };

    if stream {
        // For streaming, we still use the old single-threaded approach
        let mut files: Vec<FileEntry> = Vec::new();
        let mut total_size = 0;
        let mut current_chunk = 0;
        let mut current_chunk_files = Vec::new();

        // Walk directory tree
        for entry in WalkDir::new(base_path)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            // Get path relative to base
            let rel_path = path.strip_prefix(base_path).unwrap_or(path);
            let rel_str = rel_path.to_string_lossy();

            // Normalize path separators to forward slashes for consistent pattern matching
            #[cfg(windows)]
            let rel_str = rel_str.replace('\\', "/");

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
            #[cfg(windows)]
            let pattern_path = rel_str.replace('\\', "/");
            #[cfg(not(windows))]
            let pattern_path = rel_str.to_string();

            for pat in &final_config.ignore_patterns {
                if pat.is_match(&pattern_path) {
                    debug!("Skipping {} - matched ignore pattern", rel_str);
                    skip = true;
                    break;
                }
            }
            if skip {
                continue;
            }

            // Calculate priority score
            let mut priority = get_file_priority(
                &pattern_path,
                &final_config.ignore_patterns,
                &final_config.priority_list,
            );

            // Apply rank-based boost if available
            if let Some(ref boost_map) = recentness_boost {
                if let Some(boost) = boost_map.get(&pattern_path) {
                    priority += *boost;
                }
            }

            files.push(FileEntry {
                path: path.to_path_buf(),
                priority,
            });
        }

        // Sort files by priority (ascending) so higher priority files come last
        files.sort_by(|a, b| a.priority.cmp(&b.priority));

        // Process files in sorted order
        for file in files {
            let path = file.path;
            let rel_path = path.strip_prefix(base_path).unwrap_or(&path);
            let rel_str = rel_path.to_string_lossy();

            // Skip binary files
            if let Some(ref cfg) = config {
                if !is_text_file(&path, &cfg.binary_extensions) {
                    debug!("Skipping binary file: {}", rel_str);
                    continue;
                }
            } else if !is_text_file(&path, &[]) {
                debug!("Skipping binary file: {}", rel_str);
                continue;
            }

            // Read file content
            let content = match fs::read_to_string(&path) {
                Ok(c) => c,
                Err(e) => {
                    debug!("Failed to read {}: {}", rel_str, e);
                    continue;
                }
            };

            let size = count_size(&content, count_tokens);
            if size == 0 {
                debug!("Skipping empty file: {}", rel_str);
                continue;
            }

            // If a single file is larger than max_size, split it into multiple chunks
            if size > max_size {
                debug_file!("File exceeds chunk size, splitting into multiple chunks");
                let mut remaining = content.as_str();
                let mut part = 0;

                while !remaining.is_empty() {
                    let mut chunk_size = if count_tokens {
                        // In token mode, count words until we hit max_size
                        let mut chars = 0;
                        for (tokens, word) in remaining.split_whitespace().enumerate() {
                            if tokens + 1 > max_size {
                                break;
                            }
                            chars += word.len() + 1; // +1 for space
                        }
                        chars
                    } else {
                        max_size
                    };

                    // Ensure we make progress even if no word boundary found
                    if chunk_size == 0 {
                        chunk_size = std::cmp::min(max_size, remaining.len());
                    }

                    let (chunk, rest) =
                        remaining.split_at(std::cmp::min(chunk_size, remaining.len()));
                    remaining = rest.trim_start();

                    let chunk_files =
                        vec![(format!("{}:part{}", rel_str, part), chunk.to_string())];
                    debug_file!("Written chunk {}", part);
                    write_chunk(
                        &chunk_files,
                        part,
                        output_dir.as_deref(),
                        stream,
                        count_tokens,
                    )?;
                    part += 1;
                }
                continue;
            }

            // Check if adding this file would exceed chunk size
            if total_size + size > max_size && !current_chunk_files.is_empty() {
                // Write current chunk
                write_chunk(
                    &current_chunk_files,
                    current_chunk,
                    output_dir.as_deref(),
                    stream,
                    count_tokens,
                )?;
                debug_file!("Written chunk {}", current_chunk);
                current_chunk += 1;
                current_chunk_files.clear();
                total_size = 0;
            }

            // Add file to current chunk
            current_chunk_files.push((rel_str.to_string(), content));
            total_size += size;
        }

        // Write final chunk if any files remain
        if !current_chunk_files.is_empty() {
            write_chunk(
                &current_chunk_files,
                current_chunk,
                output_dir.as_deref(),
                stream,
                count_tokens,
            )?;
            debug_file!("Written chunk {}", current_chunk);
        }

        Ok(None)
    } else if let Some(out_dir) = output_dir {
        // Use parallel processing for non-streaming mode
        process_files_parallel(
            base_path,
            max_size,
            &out_dir,
            config.as_ref(),
            &final_config.ignore_patterns,
            &final_config.priority_list,
            recentness_boost.as_ref(),
        )?;
        Ok(Some(out_dir))
    } else {
        Ok(None)
    }
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
        debug!("Checking for config at: {}", config_path.display());
        if config_path.exists() {
            debug!("Found config at: {}", config_path.display());
            return Some(config_path);
        }
        if !current.pop() {
            debug!("No more parent directories to check");
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
