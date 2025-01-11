use anyhow::Result;
use clap::{Arg, ArgAction, Command};
use ignore::gitignore::GitignoreBuilder;
use ignore::Match;
use regex::Regex;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::fs::{create_dir_all, read_dir, File};
use std::io::{self, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command as SysCommand, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};
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
        r"^\.next/",
        r"^node_modules/",
        r"\.gitignore",
        r"pnpm-lock\.yaml",
        r"^\.vercel/",
        r"^\.turbo/",
        r"^coverage/",
        r"serialize-repo",
        r"^storybook-static/",
        r"^storybook-e2e-html-report/",
        r"^storybook-e2e-test-results/",
        r"^test-results/",
        r"\.(jpg|jpeg|png|gif|ico|woff|woff2|ttf|eot)$",
        r"\.(mp4|webm|ogg|mp3|wav|flac|aac)$",
        r"\.(pdf|doc|docx|xls|xlsx|ppt|pptx)$",
        r"\.(zip|tar|gz|tgz|rar|7z)$",
        r"\.DS_Store$",
        r"Thumbs\.db$",
        r"\.env\.local$",
        r"\.env\.development\.local$",
        r"\.env\.test\.local$",
        r"\.env\.production\.local$",
        r"test\.env$",
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
fn is_text_file(file_path: &Path, user_binary_extensions: &[String]) -> bool {
    if let Some(ext) = file_path.extension().and_then(|s| s.to_str()) {
        let dot_ext = format!(".{}", ext.to_lowercase());
        // Check both built-in and user-provided extensions
        if BINARY_FILE_EXTENSIONS.contains(&dot_ext.as_str())
            || user_binary_extensions.contains(&dot_ext)
        {
            return false;
        }
    }
    let mut f = match File::open(file_path) {
        Ok(f) => f,
        Err(_) => return false,
    };
    let mut buffer = [0u8; 4096];
    let read_bytes = match f.read(&mut buffer) {
        Ok(n) => n,
        Err(_) => return false,
    };
    for &b in &buffer[..read_bytes] {
        if b == 0 {
            return false;
        }
    }
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
    let config = config.unwrap_or_default();
    let final_cfg = build_final_config(Some(config.clone()));

    // Build an .gitignore-based ignore
    let mut gi_builder = GitignoreBuilder::new(".");
    let _ = gi_builder.add(".gitignore"); // won't fail if doesn't exist
    let gi = gi_builder.build()?;

    // If not streaming, create an output dir name
    let output_dir = if !stream {
        let checksum = get_repo_checksum(max_size);
        let path_suffix = base_path
            .and_then(|bp| bp.file_name().map(|os| os.to_string_lossy().to_string()))
            .unwrap_or_default();
        let path_suffix = if path_suffix.is_empty() {
            String::new()
        } else {
            format!("_{}", path_suffix)
        };
        let size_type = if count_tokens { "tokens" } else { "bytes" };
        let dir_name = if max_size == 0 {
            // treat 0 as infinity
            format!("{}{}", checksum, path_suffix)
        } else {
            format!("{}{}_{}{}", checksum, path_suffix, max_size, size_type)
        };
        let out_dir = std::env::current_dir()?
            .join("repo-serialized")
            .join(dir_name);
        create_dir_all(&out_dir)?;
        Some(out_dir)
    } else {
        None
    };

    let start_path = if let Some(bp) = base_path {
        std::env::current_dir()?.join(bp)
    } else {
        std::env::current_dir()?
    };

    // gather files
    let mut file_candidates = Vec::new();
    for entry in WalkDir::new(&start_path).follow_links(true) {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        if !entry.file_type().is_file() {
            continue;
        }
        let abs_path = entry.path();
        let rel_path = match abs_path.strip_prefix(std::env::current_dir()?) {
            Ok(rp) => rp.to_string_lossy().to_string(),
            Err(_) => abs_path.to_string_lossy().to_string(),
        };
        // Check gitignore
        match gi.matched_path_or_any_parents(Path::new(&rel_path), false) {
            Match::Ignore(_) => continue,
            Match::Whitelist(_) | Match::None => {}
        }
        // Check if text
        if !is_text_file(abs_path, &config.binary_extensions) {
            continue;
        }
        // Priority
        let prio = get_file_priority(
            &rel_path,
            &final_cfg.ignore_patterns,
            &final_cfg.priority_list,
        );
        if prio < 0 {
            continue;
        }
        file_candidates.push((abs_path.to_path_buf(), prio));
    }

    // sort by priority desc
    file_candidates.sort_by(|a, b| b.1.cmp(&a.1));

    let chunk_limit = if max_size == 0 { usize::MAX } else { max_size };
    let mut current_files = Vec::new();
    let mut current_size = 0usize;
    let mut total_size = 0usize;
    let mut chunk_index = 0;

    for (abs_path, _prio) in file_candidates {
        let mut content = String::new();
        File::open(&abs_path)?.read_to_string(&mut content)?;
        // file_size if we appended it alone
        let rel_path = match abs_path.strip_prefix(std::env::current_dir()?) {
            Ok(rp) => rp.to_string_lossy().to_string(),
            Err(_) => abs_path.to_string_lossy().to_string(),
        };
        let data_sample = format!(">>>> {}\n{}", rel_path, content);
        let file_size = count_size(&data_sample, count_tokens);

        if current_size + file_size > chunk_limit && !current_files.is_empty() {
            let written = write_chunk(
                &current_files,
                chunk_index,
                output_dir.as_deref(),
                stream,
                count_tokens,
            )?;
            total_size += written;
            current_files.clear();
            current_size = 0;
            chunk_index += 1;
        }
        current_files.push((rel_path, content));
        current_size += file_size;
    }

    // final flush
    if !current_files.is_empty() {
        let written = write_chunk(
            &current_files,
            chunk_index,
            output_dir.as_deref(),
            stream,
            count_tokens,
        )?;
        total_size += written;
    }

    if !stream {
        eprintln!("Total size: {}.", format_size(total_size, count_tokens));
    }

    Ok(output_dir)
}

fn main() -> Result<()> {
    let matches = Command::new("yek")
        .version("0.1.0")
        .about("Serialize a repo or subdirectory's text files into chunked text with optional token counting.")
        .arg(
            Arg::new("max_size")
                .short('t')
                .long("tokens")
                .help("Maximum tokens/bytes per chunk (defaults to Infinity if omitted or 0)")
                .value_parser(clap::value_parser!(usize))
                .default_value("0"),
        )
        .arg(
            Arg::new("path")
                .short('p')
                .long("path")
                .help("Base path to serialize (optional)")
                .value_parser(clap::value_parser!(String))
                .required(false),
        )
        .arg(
            Arg::new("model")
                .short('m')
                .long("model")
                .help("Model name, not actually used for real token counting, but accepted for parity.")
                .default_value("chatgpt-4o-latest"),
        )
        .arg(
            Arg::new("count_tokens")
                .short('c')
                .long("count-tokens")
                .help("Count tokens in a naive way rather than bytes.")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("stream")
                .short('s')
                .long("stream")
                .help("Stream output to stdout instead of writing to files.")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("configfile")
                .long("config-file")
                .help("Path to optional yek.toml config file.")
                .value_parser(clap::value_parser!(String))
                .required(false),
        )
        .get_matches();

    let max_size: usize = *matches.get_one::<usize>("max_size").unwrap_or(&0);
    let path_opt = matches.get_one::<String>("path").map(PathBuf::from);
    let count_tokens = matches.get_flag("count_tokens");
    let stream = matches.get_flag("stream");
    let config_file = matches.get_one::<String>("configfile").map(PathBuf::from);

    // If config file is provided or if there's a default yek.toml
    let config = if let Some(cf) = config_file {
        load_config_file(&cf)
    } else {
        // try default
        let def = PathBuf::from("yek.toml");
        if def.exists() {
            load_config_file(&def)
        } else {
            None
        }
    };

    if !stream {
        let from_path = path_opt
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "root".to_string());
        eprintln!(
            "Serializing repo from {} with max {}{} per chunk...",
            from_path,
            format_size(max_size, count_tokens),
            if count_tokens {
                " (tokens)"
            } else {
                " (bytes)"
            }
        );
    }

    let output = serialize_repo(max_size, path_opt.as_deref(), count_tokens, stream, config)?;

    if !stream {
        eprintln!("✨ Repository serialized successfully!");
        if let Some(dir) = output {
            if max_size == 0 {
                // single chunk
                let chunk_0 = dir.join("chunk-0.txt");
                eprintln!("Output file: {}", chunk_0.display());
            } else {
                eprintln!("Generated chunks in: {}", dir.display());
                for entry in read_dir(&dir)? {
                    let entry = entry?;
                    if entry.file_type()?.is_file() {
                        eprintln!("{}", entry.path().display());
                    }
                }
            }
        }
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
