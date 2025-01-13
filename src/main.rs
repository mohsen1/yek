use anyhow::Result;
use clap::{Arg, ArgAction, Command};
use ignore::gitignore::GitignoreBuilder;
use regex::Regex;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{self, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command as SysCommand, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{debug, info, Level};
use tracing_subscriber::FmtSubscriber;
use walkdir::WalkDir;

/// We provide an optional config that can add or override ignore patterns
/// and priority rules. All fields are optional and merged with defaults.
#[derive(Debug, Deserialize, Clone)]
struct YekConfig {
    #[serde(default)]
    ignore_patterns: IgnoreConfig,
    #[serde(default)]
    priority_rules: Vec<PriorityRule>,
    #[serde(default)]
    binary_extensions: Vec<String>,
    #[serde(default)]
    output_dir: Option<String>,
}

#[derive(Debug, Deserialize, Default, Clone)]
struct IgnoreConfig {
    #[serde(default)]
    patterns: Vec<String>,
}

#[derive(Debug, Deserialize, Clone)]
struct PriorityRule {
    score: i32,
    patterns: Vec<String>,
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
        }
    }
}

/// Internal struct that, after merging, holds the final list of ignore patterns and priorities.
struct FinalConfig {
    ignore_patterns: Vec<Regex>,
    priority_list: Vec<PriorityPattern>,
}

#[derive(Clone)]
struct PriorityPattern {
    score: i32,
    patterns: Vec<Regex>,
}

/// Default sets of priority patterns
fn default_priority_list() -> Vec<PriorityPattern> {
    vec![PriorityPattern {
        score: 50,
        patterns: vec![Regex::new(r"^src/").unwrap()],
    }]
}

/// Default sets of ignore patterns (separate from .gitignore)
fn default_ignore_patterns() -> Vec<Regex> {
    // Extra patterns from original script
    let raw = vec![
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
        r"yek.toml",
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
    // Start with default
    let mut merged_ignore = default_ignore_patterns();
    let mut merged_priority = default_priority_list();

    if let Some(user_cfg) = cfg {
        // Extend ignore
        for user_pat in user_cfg.ignore_patterns.patterns {
            if let Ok(reg) = Regex::new(&user_pat) {
                merged_ignore.push(reg);
            }
        }
        // Merge or add new priority rules.
        // We'll interpret "score" as unique: if the user has a rule with the same score, we update existing,
        // else we push as new.
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
        // re-sort priority by score desc, since we might have new ones
        merged_priority.sort_by(|a, b| b.score.cmp(&a.score));
    }

    FinalConfig {
        ignore_patterns: merged_ignore,
        priority_list: merged_priority,
    }
}

/// Check if file is text by extension or scanning first chunk for null bytes.
fn is_text_file(file_path: &Path, user_binary_extensions: &[String]) -> bool {
    debug!("Checking if file is text: {}", file_path.display());
    if let Some(ext) = file_path.extension().and_then(|s| s.to_str()) {
        let dot_ext = format!(".{}", ext.to_lowercase());
        // Check both built-in and user-provided extensions
        if BINARY_FILE_EXTENSIONS.contains(&dot_ext.as_str())
            || user_binary_extensions.contains(&dot_ext)
        {
            debug!(
                "File {} identified as binary by extension",
                file_path.display()
            );
            return false;
        }
    }
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
fn count_size(text: &str, count_tokens: bool) -> usize {
    if count_tokens {
        // extremely naive
        text.split_whitespace().count()
    } else {
        text.len()
    }
}

fn format_size(size: usize, is_tokens: bool) -> String {
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

/// Attempt to compute a short hash from git. If not available, fallback to timestamp.
fn get_repo_checksum(chunk_size: usize) -> String {
    let out = SysCommand::new("git")
        .args(["ls-files", "-c", "--exclude-standard"])
        .stderr(Stdio::null())
        .output();

    let mut hasher = Sha256::new();
    match out {
        Ok(o) => {
            if !o.status.success() {
                return fallback_timestamp();
            }
            let stdout = String::from_utf8_lossy(&o.stdout);
            let mut lines: Vec<_> = stdout
                .split('\n')
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .collect();
            lines.sort();

            for file in lines {
                let ho = SysCommand::new("git")
                    .args(["hash-object", file])
                    .stderr(Stdio::null())
                    .output();
                if let Ok(h) = ho {
                    if h.status.success() {
                        let fh = String::from_utf8_lossy(&h.stdout).trim().to_string();
                        let _ = writeln!(hasher, "{}:{}", file, fh);
                    }
                }
            }
            if chunk_size != 0 {
                let _ = write!(hasher, "{}", chunk_size);
            }
            let digest = hasher.finalize();
            let hex = format!("{:x}", digest);
            hex[..8].to_string()
        }
        Err(_) => fallback_timestamp(),
    }
}

fn fallback_timestamp() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    format!("{:x}", now)
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
        eprintln!(
            "Written chunk {} with {} files ({}).",
            index,
            files.len(),
            format_size(size, count_tokens)
        );
    }

    Ok(size)
}

/// Determine final priority of a file by scanning the priority list
/// in descending order of score. Return -1 if it's fully ignored.
fn get_file_priority(rel_str: &str, ignore_pats: &[Regex], prio_list: &[PriorityPattern]) -> i32 {
    for pat in ignore_pats {
        if pat.is_match(rel_str) {
            return -1;
        }
    }
    for prio in prio_list {
        for pat in &prio.patterns {
            if pat.is_match(rel_str) {
                return prio.score;
            }
        }
    }
    40 // fallback
}

#[derive(Debug)]
struct ConfigError {
    field: String,
    message: String,
}

fn validate_config(config: &YekConfig) -> Vec<ConfigError> {
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
            // Try to create the directory to test if we have permissions
            if let Err(e) = std::fs::create_dir_all(path) {
                errors.push(ConfigError {
                    field: "output_dir".to_string(),
                    message: format!("Cannot create output directory '{}': {}", dir, e),
                });
            } else {
                // Clean up the test directory if we created it
                let _ = std::fs::remove_dir(path);
            }
        }
    }

    errors
}

/// Merge config from a TOML file if present
fn load_config_file(path: &Path) -> Option<YekConfig> {
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

/// Find yek.toml by walking up directories
fn find_config_file(start_path: &Path) -> Option<PathBuf> {
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

#[derive(Debug)]
struct FileEntry {
    path: PathBuf,
    priority: i32,
}

/// Serialize a repository or subdir
fn serialize_repo(
    max_size: usize,
    base_path: Option<&Path>,
    count_tokens: bool,
    stream: bool,
    config: Option<YekConfig>,
    output_dir_override: Option<&Path>,
    path_prefix: Option<&str>,
) -> Result<Option<PathBuf>> {
    debug!("Starting repository serialization");
    debug!("Parameters:");
    if max_size > 0 {
        debug!("  Max size: {}", format_size(max_size, count_tokens));
    }
    debug!("  Base path: {:?}", base_path);
    debug!("  Count tokens: {}", count_tokens);
    debug!("  Stream mode: {}", stream);
    debug!("  Output dir override: {:?}", output_dir_override);

    let base_path = base_path
        .unwrap_or_else(|| Path::new("."))
        .canonicalize()
        .unwrap_or_else(|_| Path::new(".").to_path_buf());
    let mut builder = GitignoreBuilder::new(&base_path);
    let gitignore = base_path.join(".gitignore");
    if gitignore.exists() {
        debug!("Found .gitignore file at {}", gitignore.display());
        builder.add(&gitignore);
    } else {
        debug!("No .gitignore file found");
    }
    let matcher = builder.build().unwrap();

    let final_config = build_final_config(config.clone());
    debug!("Configuration processed:");
    debug!("  Ignore patterns: {}", final_config.ignore_patterns.len());
    debug!("  Priority rules: {}", final_config.priority_list.len());

    let chunk_hash = get_repo_checksum(max_size);
    let output_dir = if !stream {
        if let Some(dir) = output_dir_override {
            debug!(
                "Using output directory from command line: {}",
                dir.display()
            );
            std::fs::create_dir_all(dir)?;
            Some(dir.to_path_buf())
        } else if let Some(cfg) = &config {
            if let Some(dir) = &cfg.output_dir {
                debug!("Using output directory from config: {}", dir);
                let path = Path::new(dir);
                std::fs::create_dir_all(path)?;
                Some(path.to_path_buf())
            } else {
                debug!("Using default temporary directory");
                let dir = std::env::temp_dir().join(format!("yek-{}", chunk_hash));
                std::fs::create_dir_all(&dir)?;
                Some(dir)
            }
        } else {
            debug!("Using default temporary directory");
            let dir = std::env::temp_dir().join(format!("yek-{}", chunk_hash));
            std::fs::create_dir_all(&dir)?;
            Some(dir)
        }
    } else {
        None
    };

    let mut files: Vec<FileEntry> = Vec::new();

    // First collect all files and their priorities
    for entry in WalkDir::new(&base_path)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let rel_path = path.strip_prefix(&base_path).unwrap();
        let rel_str = rel_path.to_string_lossy();

        // Apply path prefix filter if specified
        if let Some(prefix) = path_prefix {
            if !rel_str.starts_with(prefix) {
                debug!("  Skipped: Does not match path prefix {}", prefix);
                continue;
            }
        }

        debug!("Processing file: {}", rel_str);

        // Skip if matched by gitignore
        if matcher.matched(rel_path, path.is_dir()).is_ignore() {
            debug!("  Skipped: Matched by .gitignore");
            continue;
        }

        // Skip if matched by our ignore patterns
        let priority = get_file_priority(
            &rel_str,
            &final_config.ignore_patterns,
            &final_config.priority_list,
        );
        if priority < 0 {
            debug!("  Skipped: Matched by ignore patterns");
            continue;
        }

        debug!("  Priority: {}", priority);

        // Skip binary files
        let empty_vec = vec![];
        let binary_extensions = config
            .as_ref()
            .map(|c| &c.binary_extensions)
            .unwrap_or(&empty_vec);
        if !is_text_file(path, binary_extensions) {
            debug!("  Skipped: Binary file");
            continue;
        }

        files.push(FileEntry {
            path: path.to_path_buf(),
            priority,
        });
    }

    // Sort files by priority (highest first)
    files.sort_by(|a, b| b.priority.cmp(&a.priority));

    let mut current_chunk: Vec<(String, String)> = Vec::new();
    let mut current_chunk_size = 0;
    let mut chunk_index = 0;

    // Process files in priority order
    for file in files {
        let path = file.path;
        let rel_path = path.strip_prefix(&base_path).unwrap();
        let rel_str = rel_path.to_string_lossy();

        // Read file content
        if let Ok(content) = std::fs::read_to_string(&path) {
            let size = count_size(&content, count_tokens);

            // If a single file is larger than max_size, split it into multiple chunks
            if size > max_size {
                debug!("  File exceeds chunk size, splitting into multiple chunks");
                let mut remaining = content.as_str();
                let mut part = 0;

                while !remaining.is_empty() {
                    let mut chunk_content = String::new();
                    let mut chunk_bytes = 0;

                    // Take words until we hit the size limit
                    for word in remaining.split_whitespace() {
                        let word_size = count_size(word, count_tokens);

                        // If a single word is larger than max_size, we need to split it
                        if word_size > max_size {
                            if chunk_content.is_empty() {
                                // Take a portion of the word that fits
                                let mut chars = word.chars();
                                while chunk_bytes < max_size && !chars.as_str().is_empty() {
                                    if let Some(c) = chars.next() {
                                        chunk_content.push(c);
                                        chunk_bytes += count_size(&c.to_string(), count_tokens);
                                    }
                                }
                                remaining = chars.as_str();
                            }
                            break;
                        }

                        // Normal word handling
                        if chunk_bytes + word_size + 1 > max_size {
                            break;
                        }
                        if !chunk_content.is_empty() {
                            chunk_content.push(' ');
                        }
                        chunk_content.push_str(word);
                        chunk_bytes += word_size + 1;
                    }

                    // Write current chunk
                    if !chunk_content.is_empty() {
                        current_chunk.push((
                            format!("{} (part {})", rel_str, part),
                            chunk_content.clone(),
                        ));
                        current_chunk_size += chunk_bytes;
                        part += 1;

                        // Start new chunk if needed
                        if current_chunk_size >= max_size {
                            write_chunk(
                                &current_chunk,
                                chunk_index,
                                output_dir.as_deref(),
                                stream,
                                count_tokens,
                            )?;
                            chunk_index += 1;
                            current_chunk.clear();
                            current_chunk_size = 0;
                        }
                    }

                    // Move to remaining content, handling the case where no progress was made
                    let new_remaining = if chunk_content.is_empty() {
                        // Force progress by taking the first word
                        if let Some(first_space) = remaining.find(char::is_whitespace) {
                            &remaining[first_space..].trim_start()
                        } else {
                            // No spaces found, force take some characters
                            let take_chars = std::cmp::min(remaining.len(), max_size);
                            &remaining[take_chars..].trim_start()
                        }
                    } else {
                        &remaining[chunk_content.len()..].trim_start()
                    };

                    // Ensure we're making progress
                    if new_remaining.len() == remaining.len() {
                        // Emergency break to prevent infinite loop
                        debug!(
                            "Warning: Unable to make progress in splitting file {}",
                            rel_str
                        );
                        break;
                    }
                    remaining = new_remaining;
                }
            } else {
                // Regular file handling
                if current_chunk_size + size > max_size && !current_chunk.is_empty() {
                    // Write current chunk and start new one
                    write_chunk(
                        &current_chunk,
                        chunk_index,
                        output_dir.as_deref(),
                        stream,
                        count_tokens,
                    )?;
                    chunk_index += 1;
                    current_chunk.clear();
                    current_chunk_size = 0;
                }

                current_chunk.push((rel_str.to_string(), content));
                current_chunk_size += size;
            }
        }
    }

    // Write any remaining files in the last chunk
    if !current_chunk.is_empty() {
        write_chunk(
            &current_chunk,
            chunk_index,
            output_dir.as_deref(),
            stream,
            count_tokens,
        )?;
    }

    Ok(output_dir)
}

fn main() -> Result<()> {
    let matches = Command::new("yek")
        .about("Serialize repository content for LLM context")
        .arg(
            Arg::new("path")
                .help("Path to repository")
                .default_value(".")
                .index(1),
        )
        .arg(
            Arg::new("max-size")
                .help("Maximum size in MB")
                .short('x')
                .long("max-size")
                .value_parser(clap::value_parser!(usize))
                .default_value("10"),
        )
        .arg(
            Arg::new("config")
                .help("Path to config file")
                .short('c')
                .long("config"),
        )
        .arg(
            Arg::new("output-dir")
                .help("Directory to write output files (overrides config file)")
                .short('o')
                .long("output-dir"),
        )
        .arg(
            Arg::new("stream")
                .help("Stream output to stdout instead of writing to files")
                .short('s')
                .long("stream")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("tokens")
                .short('k')
                .long("tokens")
                .help("Count in tokens instead of bytes")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("path-prefix")
                .short('p')
                .long("path-prefix")
                .help("Only process files under this path prefix")
                .value_name("PREFIX"),
        )
        .arg(
            Arg::new("debug")
                .help("Enable debug logging")
                .short('v')
                .long("debug")
                .action(ArgAction::SetTrue),
        )
        .get_matches();

    // Initialize logging based on debug flag
    FmtSubscriber::builder()
        .with_max_level(if matches.get_flag("debug") {
            Level::DEBUG
        } else {
            Level::INFO
        })
        .with_target(false)
        .with_thread_ids(false)
        .with_thread_names(false)
        .with_file(false)
        .with_line_number(false)
        .with_level(true)
        .with_ansi(true)
        .with_timer(tracing_subscriber::fmt::time::LocalTime::new(
            time::format_description::parse("[hour]:[minute]:[second]").unwrap(),
        ))
        .compact()
        .init();

    debug!("Starting yek with debug logging enabled");

    let path = matches
        .get_one::<String>("path")
        .map(|s| s.as_str())
        .unwrap_or(".");
    let count_tokens = matches.get_flag("tokens");
    let max_size = matches
        .get_one::<usize>("max-size")
        .map(|s| if count_tokens { *s } else { s * 1024 * 1024 })
        .unwrap_or(10 * 1024 * 1024);
    let stream = matches.get_flag("stream");
    let output_dir = matches.get_one::<String>("output-dir").map(Path::new);
    let path_prefix = matches.get_one::<String>("path-prefix").map(|s| s.as_str());

    debug!("CLI Arguments:");
    debug!("  Repository path: {}", path);
    debug!("  Maximum size: {} bytes", max_size);
    debug!("  Stream mode: {}", stream);
    debug!("  Token counting mode: {}", count_tokens);
    debug!("  Output directory: {:?}", output_dir);

    let config_path = matches
        .get_one::<String>("config")
        .map(PathBuf::from)
        .or_else(|| find_config_file(Path::new(path)));

    let config = config_path.and_then(|p| load_config_file(&p));
    debug!("Configuration:");
    debug!("  Config file loaded: {}", config.is_some());
    if let Some(cfg) = &config {
        debug!("  Ignore patterns: {}", cfg.ignore_patterns.patterns.len());
        debug!("  Priority rules: {}", cfg.priority_rules.len());
        debug!("  Binary extensions: {}", cfg.binary_extensions.len());
        debug!("  Output directory: {:?}", cfg.output_dir);
    }

    if let Some(output_path) = serialize_repo(
        max_size,
        Some(Path::new(path)),
        count_tokens,
        stream,
        config,
        output_dir,
        path_prefix,
    )? {
        info!("Output written to: {}", output_path.display());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_file(dir: &Path, name: &str, content: &[u8]) -> PathBuf {
        let path = dir.join(name);
        fs::write(&path, content).unwrap();
        path
    }

    #[test]
    fn test_is_text_file() {
        let temp = TempDir::new().unwrap();
        let text_file = create_test_file(temp.path(), "test.txt", b"Hello World");
        let binary_file = create_test_file(temp.path(), "test.jpg", &[0u8; 100]);

        assert!(is_text_file(&text_file, &[]));
        assert!(!is_text_file(&binary_file, &[]));
        assert!(!is_text_file(&text_file, &[".txt".to_string()]));
    }

    #[test]
    fn test_count_size() {
        let text = "Hello World\nThis is a test";
        assert_eq!(count_size(text, false), text.len());
        assert_eq!(count_size(text, true), 6); // "Hello", "World", "This", "is", "a", "test"
    }

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(1000, true), "1000 tokens");
        assert_eq!(format_size(1000, false), "1000.0 B");
        assert_eq!(format_size(1500, false), "1.5 KB");
        assert_eq!(format_size(1_500_000, false), "1.4 MB"); // Fixed to match actual calculation
    }

    #[test]
    fn test_config_merge() {
        let user_config = YekConfig {
            ignore_patterns: IgnoreConfig {
                patterns: vec!["test/".to_string()],
            },
            priority_rules: vec![PriorityRule {
                score: 100,
                patterns: vec!["src/.*".to_string()],
            }],
            binary_extensions: vec![".custom".to_string()],
            output_dir: None,
        };

        let final_cfg = build_final_config(Some(user_config));

        // Test ignore patterns
        assert!(final_cfg
            .ignore_patterns
            .iter()
            .any(|p| p.as_str() == "test/"));

        // Test priority rules
        let src_file = "src/main.rs";
        let other_file = "other/file.rs";
        let prio_src = get_file_priority(src_file, &[], &final_cfg.priority_list);
        let prio_other = get_file_priority(other_file, &[], &final_cfg.priority_list);
        assert!(prio_src > prio_other);
    }

    #[test]
    fn test_config_validation() {
        // Test invalid regex patterns
        let config = YekConfig {
            ignore_patterns: IgnoreConfig {
                patterns: vec!["[".to_string()], // Invalid regex
            },
            priority_rules: vec![],
            binary_extensions: vec![],
            output_dir: None,
        };
        let errors = validate_config(&config);
        assert!(!errors.is_empty());
        assert!(errors[0].message.contains("Invalid regex pattern"));

        // Test invalid priority score
        let config = YekConfig {
            ignore_patterns: IgnoreConfig { patterns: vec![] },
            priority_rules: vec![PriorityRule {
                score: -1,
                patterns: vec![".*".to_string()],
            }],
            binary_extensions: vec![],
            output_dir: None,
        };
        let errors = validate_config(&config);
        assert!(!errors.is_empty());
        assert!(errors[0].message.contains("must be between 0 and 1000"));

        // Test valid config
        let config = YekConfig {
            ignore_patterns: IgnoreConfig {
                patterns: vec![".*\\.txt$".to_string()],
            },
            priority_rules: vec![PriorityRule {
                score: 100,
                patterns: vec!["^src/.*".to_string()],
            }],
            binary_extensions: vec![],
            output_dir: None,
        };
        let errors = validate_config(&config);
        assert!(errors.is_empty());
    }
}
