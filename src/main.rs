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
struct LlmSerializeConfig {
    #[serde(default)]
    ignore_patterns: IgnoreConfig,
    #[serde(default)]
    priority_rules: Vec<PriorityRule>,
    #[serde(default)]
    #[allow(dead_code)]
    binary_extensions: Vec<String>,
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
#[allow(dead_code)]
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
impl Default for LlmSerializeConfig {
    fn default() -> Self {
        LlmSerializeConfig {
            ignore_patterns: IgnoreConfig::default(),
            priority_rules: vec![
                // Default fallback - everything has same priority
                PriorityRule {
                    score: 1,
                    patterns: vec![".*".to_string()],
                },
            ],
            binary_extensions: Vec::new(), // User extensions only, we'll combine with BINARY_FILE_EXTENSIONS
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
    vec![
        PriorityPattern {
            score: 100,
            patterns: vec![Regex::new(r"^src/lib/").unwrap()],
        },
        PriorityPattern {
            score: 90,
            patterns: vec![
                Regex::new(r"^src/utils/").unwrap(),
                Regex::new(r"^src/contexts/").unwrap(),
                Regex::new(r"^src/hooks/").unwrap(),
                Regex::new(r"^src/constant/").unwrap(),
                Regex::new(r"^src/shared/").unwrap(),
            ],
        },
        PriorityPattern {
            score: 80,
            patterns: vec![
                Regex::new(r"^src/app/").unwrap(),
                Regex::new(r"^src/components/").unwrap(),
            ],
        },
        PriorityPattern {
            score: 70,
            patterns: vec![
                Regex::new(r"^src/styles/").unwrap(),
                Regex::new(r"^src/design-system/").unwrap(),
            ],
        },
        PriorityPattern {
            score: 60,
            patterns: vec![
                Regex::new(r"^src/tests/").unwrap(),
                Regex::new(r"^src/e2e/").unwrap(),
                Regex::new(r"^src/__tests__/").unwrap(),
            ],
        },
        PriorityPattern {
            score: 50,
            patterns: vec![
                Regex::new(r"^src/").unwrap(),
                Regex::new(r"^docs/").unwrap(),
            ],
        },
        // default fallback
        PriorityPattern {
            score: 40,
            patterns: vec![Regex::new(r".*").unwrap()],
        },
    ]
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
fn build_final_config(cfg: Option<LlmSerializeConfig>) -> FinalConfig {
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
#[allow(dead_code)]
fn is_text_file(file_path: &Path, user_binary_extensions: &[String]) -> bool {
    debug!("Checking if file is text: {}", file_path.display());
    if let Some(ext) = file_path.extension().and_then(|s| s.to_str()) {
        let dot_ext = format!(".{}", ext.to_lowercase());
        // Check both built-in and user-provided extensions
        if BINARY_FILE_EXTENSIONS.contains(&dot_ext.as_str())
            || user_binary_extensions.contains(&dot_ext)
        {
            debug!("File {} identified as binary by extension", file_path.display());
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
            debug!("File {} identified as binary by content", file_path.display());
            return false;
        }
    }
    debug!("File {} identified as text", file_path.display());
    true
}

/// Naive token counting or raw byte length
#[allow(dead_code)]
fn count_size(text: &str, count_tokens: bool) -> usize {
    if count_tokens {
        // extremely naive
        text.split_whitespace().count()
    } else {
        text.len()
    }
}

#[allow(dead_code)]
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
#[allow(dead_code)]
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

#[allow(dead_code)]
fn fallback_timestamp() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    format!("{:x}", now)
}

/// Write chunk to file or stdout
#[allow(dead_code)]
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

/// Merge config from a TOML file if present
fn load_config_file(path: &Path) -> Option<LlmSerializeConfig> {
    let content = std::fs::read_to_string(path).ok()?;
    let parsed = toml::from_str::<LlmSerializeConfig>(&content).ok()?;
    Some(parsed)
}

/// Serialize a repository or subdir
fn serialize_repo(
    max_size: usize,
    base_path: Option<&Path>,
    count_tokens: bool,
    stream: bool,
    config: Option<LlmSerializeConfig>,
) -> Result<Option<PathBuf>> {
    debug!("Starting repository serialization");
    debug!("Parameters:");
    debug!("  Max size: {}", format_size(max_size, count_tokens));
    debug!("  Base path: {:?}", base_path);
    debug!("  Count tokens: {}", count_tokens);
    debug!("  Stream mode: {}", stream);

    let base_path = base_path.unwrap_or_else(|| Path::new("."));
    let mut builder = GitignoreBuilder::new(base_path);
    let gitignore = Path::new(".gitignore");
    if gitignore.exists() {
        debug!("Found .gitignore file at {}", gitignore.display());
        builder.add(gitignore);
    } else {
        debug!("No .gitignore file found");
    }
    let matcher = builder.build().unwrap();

    let final_config = build_final_config(config);
    debug!("Configuration processed:");
    debug!("  Ignore patterns: {}", final_config.ignore_patterns.len());
    debug!("  Priority rules: {}", final_config.priority_list.len());

    let _files: Vec<(String, String)> = Vec::new();
    let _total_size = 0;

    for entry in WalkDir::new(base_path)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let rel_path = path.strip_prefix(base_path).unwrap();
        let rel_str = rel_path.to_string_lossy();

        debug!("Processing file: {}", rel_str);

        // Skip if matched by gitignore
        if matcher.matched(rel_path, false).is_ignore() {
            debug!("  Skipped: Matched by .gitignore");
            continue;
        }

        // Skip if matched by our ignore patterns
        let priority = get_file_priority(&rel_str, &final_config.ignore_patterns, &final_config.priority_list);
        if priority < 0 {
            debug!("  Skipped: Matched by ignore patterns");
            continue;
        }

        debug!("  Priority: {}", priority);

        // ... rest of the function ...
    }

    debug!("Repository serialization completed");
    Ok(None)
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
                .short('s')
                .long("max-size")
                .default_value("10"),
        )
        .arg(
            Arg::new("config")
                .help("Path to config file")
                .short('c')
                .long("config"),
        )
        .arg(
            Arg::new("stream")
                .help("Stream output to stdout instead of writing to file")
                .short('t')
                .long("stream")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("tokens")
                .help("Count tokens instead of bytes")
                .short('k')
                .long("tokens")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("debug")
                .help("Enable debug logging")
                .short('d')
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
        .with_timer(tracing_subscriber::fmt::time::LocalTime::new(time::format_description::parse("[hour]:[minute]:[second]").unwrap()))
        .compact()
        .init();

    debug!("Starting yek with debug logging enabled");

    let path = matches.get_one::<String>("path").unwrap();
    let max_size: usize = matches
        .get_one::<String>("max-size")
        .unwrap()
        .parse()
        .unwrap_or(10)
        * 1024
        * 1024;
    let stream = matches.get_flag("stream");
    let count_tokens = matches.get_flag("tokens");

    debug!("CLI Arguments:");
    debug!("  Repository path: {}", path);
    debug!("  Maximum size: {} bytes", max_size);
    debug!("  Stream mode: {}", stream);
    debug!("  Token counting mode: {}", count_tokens);

    let config = matches
        .get_one::<String>("config")
        .map(|p| load_config_file(Path::new(p)))
        .flatten();
    debug!("Configuration:");
    debug!("  Config file loaded: {}", config.is_some());
    if let Some(cfg) = &config {
        debug!("  Ignore patterns: {}", cfg.ignore_patterns.patterns.len());
        debug!("  Priority rules: {}", cfg.priority_rules.len());
        debug!("  Binary extensions: {}", cfg.binary_extensions.len());
    }

    if let Some(output_path) = serialize_repo(max_size, Some(Path::new(path)), count_tokens, stream, config)? {
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
        let user_config = LlmSerializeConfig {
            ignore_patterns: IgnoreConfig {
                patterns: vec!["test/".to_string()],
            },
            priority_rules: vec![PriorityRule {
                score: 100,
                patterns: vec!["src/.*".to_string()],
            }],
            binary_extensions: vec![".custom".to_string()],
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
}
