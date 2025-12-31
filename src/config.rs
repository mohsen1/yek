use anyhow::{anyhow, Result};
use bytesize::ByteSize;
use clap_config_file::ClapConfigFile;
use sha2::{Digest, Sha256};
use std::io::{self, BufRead, BufReader, IsTerminal};
use std::{fs, path::Path, process::Command, str::FromStr, time::UNIX_EPOCH};

use crate::{
    defaults::{BINARY_FILE_EXTENSIONS, DEFAULT_IGNORE_PATTERNS, DEFAULT_OUTPUT_TEMPLATE},
    priority::PriorityRule,
};

#[derive(Clone, Debug, Default, clap::ValueEnum, serde::Serialize, serde::Deserialize)]
pub enum ConfigFormat {
    #[default]
    Toml,
    Yaml,
    Json,
}

#[derive(ClapConfigFile, Clone)]
#[config_file_name = "yek"]
#[config_file_formats = "toml,yaml,json"]
pub struct YekConfig {
    /// Input files and/or directories to process
    #[config_arg(positional)]
    pub input_paths: Vec<String>,

    /// Print version of yek
    #[config_arg(long = "version", short = 'V')]
    pub version: bool,

    /// Update yek to the latest version
    #[config_arg(long = "update")]
    pub update: bool,

    /// Max size per chunk. e.g. "10MB" or "128K" or when using token counting mode, "100" or "128K"
    #[config_arg(default_value = "10MB")]
    pub max_size: String,

    /// Use token mode instead of byte mode
    #[config_arg()]
    pub tokens: String,

    /// Enable JSON output
    #[config_arg()]
    pub json: bool,

    /// Enable debug output
    #[config_arg()]
    pub debug: bool,

    /// Include line numbers in output
    #[config_arg(long = "line-numbers")]
    pub line_numbers: bool,

    /// Output directory. If none is provided & stdout is a TTY, we pick a temp dir
    #[config_arg()]
    pub output_dir: Option<String>,

    /// Output filename. If provided, write output to this file in current directory
    #[config_arg(long = "output-name")]
    pub output_name: Option<String>,

    /// Output template. Defaults to ">>>> FILE_PATH\nFILE_CONTENT"
    #[config_arg()]
    pub output_template: Option<String>,

    /// Ignore patterns
    #[config_arg(long = "ignore-patterns", multi_value_behavior = "extend")]
    pub ignore_patterns: Vec<String>,

    /// Unignore patterns. Yek has some built-in ignore patterns, but you can override them here.
    #[config_arg(long = "unignore-patterns", multi_value_behavior = "extend")]
    pub unignore_patterns: Vec<String>,

    /// Priority rules
    #[config_arg(accept_from = "config_only")]
    pub priority_rules: Vec<PriorityRule>,

    /// Binary file extensions to ignore
    #[config_arg(accept_from = "config_only", default_value = BINARY_FILE_EXTENSIONS)]
    pub binary_extensions: Vec<String>,

    /// Maximum additional boost from Git commit times (0..1000)
    #[config_arg(accept_from = "config_only")]
    pub git_boost_max: Option<i32>,

    /// Category-based priority weights
    #[config_arg(accept_from = "config_only")]
    pub category_weights: Option<crate::category::CategoryWeights>,

    /// Include directory tree header in output (incompatible with JSON output)
    #[config_arg(long = "tree-header", short = 't')]
    pub tree_header: bool,

    /// Show only the directory tree (no file contents, incompatible with JSON output)
    #[config_arg(long = "tree-only")]
    pub tree_only: bool,

    /// True if we should stream output to stdout (computed)
    pub stream: bool,

    /// True if we should count tokens, not bytes (computed)
    pub token_mode: bool,

    /// Final resolved output file path (only used if not streaming)
    pub output_file_full_path: Option<String>,

    /// Maximum depth to search for Git commit times
    #[config_arg(accept_from = "config_only", default_value = "100")]
    pub max_git_depth: i32,
}

/// Provide defaults so tests or other callers can create a baseline YekConfig easily.
impl Default for YekConfig {
    fn default() -> Self {
        Self {
            input_paths: Vec::new(),
            version: false,
            update: false,
            max_size: "10MB".to_string(),
            tokens: String::new(),
            json: false,
            debug: false,
            line_numbers: false,
            output_dir: None,
            output_name: None,
            output_template: Some(DEFAULT_OUTPUT_TEMPLATE.to_string()),
            ignore_patterns: Vec::new(),
            unignore_patterns: Vec::new(),
            priority_rules: Vec::new(),
            binary_extensions: BINARY_FILE_EXTENSIONS
                .iter()
                .map(|s| s.to_string())
                .collect(),
            git_boost_max: Some(100),
            category_weights: None,

            // computed fields
            tree_header: false,
            tree_only: false,
            stream: false,
            token_mode: false,
            output_file_full_path: None,
            max_git_depth: 100,
        }
    }
}

impl YekConfig {
    pub fn extend_config_with_defaults(input_paths: Vec<String>, output_dir: String) -> Self {
        YekConfig {
            input_paths,
            output_dir: Some(output_dir),
            ..Default::default()
        }
    }

    /// Read input paths from stdin, filtering out empty lines and trimming whitespace
    fn read_input_paths_from_stdin(&self) -> Result<Vec<String>> {
        let stdin = io::stdin();
        let reader = BufReader::new(stdin.lock());
        let mut paths = Vec::new();

        for line in reader.lines() {
            let line = line?;
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                paths.push(trimmed.to_string());
            }
        }

        Ok(paths)
    }

    /// Ensure output directory exists and is valid. Returns the resolved output directory path.
    pub fn ensure_output_dir(&self) -> Result<String> {
        if self.stream {
            return Ok(String::new());
        }

        let output_dir = if let Some(dir) = &self.output_dir {
            dir.clone()
        } else {
            let temp_dir = std::env::temp_dir().join("yek-output");
            temp_dir.to_string_lossy().to_string()
        };

        let path = Path::new(&output_dir);
        if path.exists() && !path.is_dir() {
            return Err(anyhow!(
                "output_dir: '{}' exists but is not a directory",
                output_dir
            ));
        }

        std::fs::create_dir_all(path)
            .map_err(|e| anyhow!("output_dir: cannot create '{}': {}", output_dir, e))?;

        Ok(output_dir)
    }

    /// Parse from CLI + config file, fill in computed fields, and validate.
    pub fn init_config() -> Self {
        // 1) parse from CLI and optional config file:
        let (mut cfg, _config_path, _config_format) = YekConfig::parse_info();

        // Handle version flag
        if cfg.version {
            println!("{}", env!("CARGO_PKG_VERSION"));
            std::process::exit(0);
        }

        // Handle update flag
        if cfg.update {
            match cfg.perform_update() {
                Ok(()) => std::process::exit(0),
                Err(e) => {
                    eprintln!("Error updating yek: {}", e);
                    std::process::exit(1);
                }
            }
        }

        // 2) compute derived fields:
        cfg.token_mode = !cfg.tokens.is_empty();
        let force_tty = std::env::var("FORCE_TTY").is_ok();

        cfg.stream = !std::io::stdout().is_terminal() && !force_tty;

        // Handle default for output_template if not provided
        if cfg.output_template.is_none() {
            cfg.output_template = Some(DEFAULT_OUTPUT_TEMPLATE.to_string());
        }

        // Check if we should read input paths from stdin
        if cfg.input_paths.is_empty() {
            if !std::io::stdin().is_terminal() {
                // Read file paths from stdin (one per line)
                match cfg.read_input_paths_from_stdin() {
                    Ok(stdin_paths) => {
                        if !stdin_paths.is_empty() {
                            cfg.input_paths = stdin_paths;
                        } else {
                            // stdin was empty, default to current dir
                            cfg.input_paths.push(".".to_string());
                        }
                    }
                    Err(e) => {
                        eprintln!("Warning: Failed to read from stdin: {}", e);
                        cfg.input_paths.push(".".to_string());
                    }
                }
            } else {
                // No stdin input, default to current dir
                cfg.input_paths.push(".".to_string());
            }
        }

        // Extend binary extensions with the built-in list:
        let mut merged_bins = BINARY_FILE_EXTENSIONS
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>();
        merged_bins.append(&mut cfg.binary_extensions);
        cfg.binary_extensions = merged_bins
            .into_iter()
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        // Always start with default ignore patterns, then add user's:
        let mut ignore = DEFAULT_IGNORE_PATTERNS
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>();
        ignore.extend(cfg.ignore_patterns);
        cfg.ignore_patterns = ignore;

        // Apply unignore patterns (turn them into negative globs "!…")
        cfg.ignore_patterns
            .extend(cfg.unignore_patterns.iter().map(|pat| format!("!{}", pat)));

        // Handle output directory setup
        if !cfg.stream {
            match cfg.ensure_output_dir() {
                Ok(dir) => cfg.output_dir = Some(dir),
                Err(e) => {
                    eprintln!("Warning: Failed to create output directory: {}", e);
                    cfg.stream = true; // Fall back to streaming mode
                }
            }
        }

        // By default, we start with no final output_file_full_path:
        cfg.output_file_full_path = None;

        // 3) Validate
        if let Err(e) = cfg.validate() {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }

        cfg
    }

    /// Compute a quick checksum for the input paths (files and directories).
    /// For directories, it uses the top-level listing. For files, it uses the file metadata.
    pub fn get_checksum(input_paths: &[String]) -> String {
        let mut hasher = Sha256::new();
        for path_str in input_paths {
            let base_path = Path::new(path_str);
            if !base_path.exists() {
                continue;
            }

            // If it's a file, hash the file metadata directly
            if base_path.is_file() {
                if let Ok(meta) = fs::metadata(base_path) {
                    hasher.update(path_str.as_bytes());
                    hasher.update(meta.len().to_le_bytes());

                    if let Ok(mod_time) = meta.modified() {
                        if let Ok(dur) = mod_time.duration_since(UNIX_EPOCH) {
                            hasher.update(dur.as_secs().to_le_bytes());
                            hasher.update(dur.subsec_nanos().to_le_bytes());
                        }
                    }
                }
                continue;
            }

            // If it's a directory, hash its contents
            let entries = match fs::read_dir(base_path) {
                Ok(iter) => iter.filter_map(|e| e.ok()).collect::<Vec<_>>(),
                Err(_) => continue,
            };

            // Sort deterministically by path name
            let mut sorted = entries;
            sorted.sort_by_key(|a| a.path());

            for entry in sorted {
                let p = entry.path();
                if let Ok(meta) = fs::metadata(&p) {
                    let path_str = p.to_string_lossy();
                    hasher.update(path_str.as_bytes());
                    hasher.update(meta.len().to_le_bytes());

                    if let Ok(mod_time) = meta.modified() {
                        if let Ok(dur) = mod_time.duration_since(UNIX_EPOCH) {
                            hasher.update(dur.as_secs().to_le_bytes());
                            hasher.update(dur.subsec_nanos().to_le_bytes());
                        }
                    }
                }
            }
        }
        let result = hasher.finalize();
        // Convert the 32-byte result to hex, but only keep the first 8 characters
        let hex = format!("{:x}", result);
        hex[..8].to_owned()
    }

    /// Validate the final config.
    pub fn validate(&self) -> Result<()> {
        let template = self
            .output_template
            .as_ref()
            .ok_or_else(|| anyhow!("output_template: must be provided"))?;

        if !template.contains("FILE_PATH") || !template.contains("FILE_CONTENT") {
            return Err(anyhow!(
                "output_template: must contain FILE_PATH and FILE_CONTENT"
            ));
        }

        if self.max_size == "0" {
            return Err(anyhow!("max_size: cannot be 0"));
        }

        if !self.token_mode {
            ByteSize::from_str(&self.max_size)
                .map_err(|e| anyhow!("max_size: Invalid size format: {}", e))?;
        } else if self.tokens.to_lowercase().ends_with('k') {
            // Use UTF-8 aware slicing to handle emojis and other multi-byte characters
            let chars: Vec<char> = self.tokens.chars().collect();
            if chars.len() > 1 {
                let val = chars[..chars.len() - 1]
                    .iter()
                    .collect::<String>()
                    .trim()
                    .parse::<usize>()
                    .map_err(|e| anyhow!("tokens: Invalid token size: {}", e))?;
                if val == 0 {
                    return Err(anyhow!("tokens: cannot be 0"));
                }
            } else {
                return Err(anyhow!("tokens: Invalid token format: {}", self.tokens));
            }
        } else if !self.tokens.is_empty() {
            // parse as integer
            let val = self
                .tokens
                .parse::<usize>()
                .map_err(|e| anyhow!("tokens: Invalid token size: {}", e))?;
            if val == 0 {
                return Err(anyhow!("tokens: cannot be 0"));
            }
        }

        // If not streaming, validate output directory
        if !self.stream {
            self.ensure_output_dir()?;
        }

        // Validate ignore patterns
        for pattern in &self.ignore_patterns {
            glob::Pattern::new(pattern)
                .map_err(|e| anyhow!("ignore_patterns: Invalid pattern '{}': {}", pattern, e))?;
        }

        // Validate priority rules
        for rule in &self.priority_rules {
            if rule.score < 0 || rule.score > 1000 {
                return Err(anyhow!(
                    "priority_rules: Priority score {} must be between 0 and 1000",
                    rule.score
                ));
            }
            glob::Pattern::new(&rule.pattern).map_err(|e| {
                anyhow!("priority_rules: Invalid pattern '{}': {}", rule.pattern, e)
            })?;
        }

        // Validate tree options are mutually exclusive
        if self.tree_header && self.tree_only {
            return Err(anyhow!("tree_header and tree_only cannot both be enabled"));
        }

        // Validate JSON output is not used with tree modes
        if self.json && self.tree_header {
            return Err(anyhow!("JSON output not supported with tree header mode"));
        }

        if self.json && self.tree_only {
            return Err(anyhow!("JSON output not supported in tree-only mode"));
        }

        Ok(())
    }

    /// Update yek to the latest version by downloading and replacing the current binary
    pub fn perform_update(&self) -> Result<()> {
        const REPO_OWNER: &str = "mohsen1";
        const REPO_NAME: &str = "yek";

        println!("Checking for latest version...");

        // Get the current executable path
        let current_exe = std::env::current_exe()
            .map_err(|e| anyhow!("Failed to get current executable path: {}", e))?;

        if !current_exe.exists() {
            return Err(anyhow!("Current executable path does not exist"));
        }

        // Check if the current executable is writable
        let metadata = fs::metadata(&current_exe)?;
        if metadata.permissions().readonly() {
            return Err(anyhow!("Cannot update: current executable is not writable. Try running with elevated permissions or install to a writable location."));
        }

        // Determine target architecture
        let target = Self::get_target_triple()?;
        let asset_name = format!("yek-{}.tar.gz", target);

        println!("Fetching release info for target: {}", target);

        // Get latest release info from GitHub API
        let releases_url = format!(
            "https://api.github.com/repos/{}/{}/releases/latest",
            REPO_OWNER, REPO_NAME
        );
        let releases_output = Command::new("curl")
            .args(["-s", &releases_url])
            .output()
            .map_err(|e| anyhow!("Failed to execute curl command: {}. Is curl installed?", e))?;

        if !releases_output.status.success() {
            let stderr = String::from_utf8_lossy(&releases_output.stderr);
            return Err(anyhow!("Failed to fetch release info: {}", stderr));
        }

        let release_json = String::from_utf8_lossy(&releases_output.stdout);

        // Parse JSON to find download URL (simple string parsing to avoid adding serde_json dependency)
        let download_url = Self::extract_download_url(&release_json, &asset_name)?;

        // Get the new version tag
        let new_version = Self::extract_version_tag(&release_json)?;
        let current_version = env!("CARGO_PKG_VERSION");

        println!("Current version: {}", current_version);
        println!("Latest version: {}", new_version);

        if new_version == current_version {
            println!("You are already running the latest version!");
            return Ok(());
        }

        println!("Downloading update from: {}", download_url);

        // Create temp directory for download
        let temp_dir = std::env::temp_dir().join(format!("yek-update-{}", new_version));
        fs::create_dir_all(&temp_dir)?;

        let archive_path = temp_dir.join(&asset_name);

        // Download the archive
        let download_output = Command::new("curl")
            .args(["-L", "-o"])
            .arg(&archive_path)
            .arg(&download_url)
            .output()
            .map_err(|e| anyhow!("Failed to download update: {}", e))?;

        if !download_output.status.success() {
            let stderr = String::from_utf8_lossy(&download_output.stderr);
            return Err(anyhow!("Failed to download update: {}", stderr));
        }

        // Extract the archive
        println!("Extracting update...");
        let extract_output = Command::new("tar")
            .args(["xzf"])
            .arg(&archive_path)
            .current_dir(&temp_dir)
            .output()
            .map_err(|e| anyhow!("Failed to extract archive: {}. Is tar installed?", e))?;

        if !extract_output.status.success() {
            let stderr = String::from_utf8_lossy(&extract_output.stderr);
            return Err(anyhow!("Failed to extract archive: {}", stderr));
        }

        // Find the new binary
        let extracted_dir = temp_dir.join(format!("yek-{}", target));
        let new_binary = extracted_dir.join("yek");

        if !new_binary.exists() {
            return Err(anyhow!("Updated binary not found in extracted archive"));
        }

        // Replace the current binary
        println!("Installing update...");

        // Create backup of current binary
        let backup_path = format!("{}.backup", current_exe.to_string_lossy());
        fs::copy(&current_exe, &backup_path)?;

        // Replace with new binary
        match fs::copy(&new_binary, &current_exe) {
            Ok(_) => {
                // Make the new binary executable (Unix-like systems)
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let mut perms = fs::metadata(&current_exe)?.permissions();
                    perms.set_mode(0o755);
                    fs::set_permissions(&current_exe, perms)?;
                }

                // Remove backup on success
                let _ = fs::remove_file(&backup_path);

                println!(
                    "Successfully updated yek from {} to {}!",
                    current_version, new_version
                );
                println!("Update complete! You can now run yek with the new version.");
            }
            Err(e) => {
                // Restore from backup on failure
                let _ = fs::copy(&backup_path, &current_exe);
                let _ = fs::remove_file(&backup_path);
                return Err(anyhow!("Failed to replace binary: {}", e));
            }
        }

        // Cleanup temp directory
        let _ = fs::remove_dir_all(&temp_dir);

        Ok(())
    }

    /// Determine the target triple for the current platform
    pub fn get_target_triple() -> Result<String> {
        let os = std::env::consts::OS;
        let arch = std::env::consts::ARCH;

        let target = match (os, arch) {
            ("linux", "x86_64") => {
                // Try to detect if we should use musl or gnu
                // Default to musl for better compatibility
                "x86_64-unknown-linux-musl"
            }
            ("linux", "aarch64") => "aarch64-unknown-linux-musl",
            ("macos", "x86_64") => "x86_64-apple-darwin",
            ("macos", "aarch64") => "aarch64-apple-darwin",
            ("windows", "x86_64") => "x86_64-pc-windows-msvc",
            ("windows", "aarch64") => "aarch64-pc-windows-msvc",
            _ => return Err(anyhow!("Unsupported platform: {} {}", os, arch)),
        };

        Ok(target.to_string())
    }

    /// Extract download URL from GitHub releases API JSON response
    pub fn extract_download_url(json: &str, asset_name: &str) -> Result<String> {
        // Simple JSON parsing to find the browser_download_url for our asset
        let lines: Vec<&str> = json.lines().collect();
        let mut found_asset = false;

        for line in lines.iter() {
            // Look for the asset name
            if line.contains(&format!("\"name\": \"{}\"", asset_name)) {
                found_asset = true;
                continue;
            }

            // If we found our asset, look for the download URL in nearby lines
            if found_asset && line.contains("browser_download_url") {
                if let Some(url_start) = line.find("https://") {
                    if let Some(url_end) = line[url_start..].find('"') {
                        let url = &line[url_start..url_start + url_end];
                        return Ok(url.to_string());
                    }
                }
            }
        }

        Err(anyhow!(
            "Could not find download URL for asset: {}",
            asset_name
        ))
    }

    /// Extract version tag from GitHub releases API JSON response
    pub fn extract_version_tag(json: &str) -> Result<String> {
        // Look for "tag_name": "v1.2.3"
        for line in json.lines() {
            if line.contains("\"tag_name\":") {
                // Find the colon after tag_name
                if let Some(colon_pos) = line.find(':') {
                    let after_colon = &line[colon_pos + 1..];
                    // Find the first quote after the colon
                    if let Some(first_quote) = after_colon.find('"') {
                        let value_start = first_quote + 1;
                        // Find the closing quote
                        if let Some(second_quote) = after_colon[value_start..].find('"') {
                            let tag = &after_colon[value_start..value_start + second_quote];
                            // Remove 'v' prefix if present
                            let version = if let Some(stripped) = tag.strip_prefix('v') {
                                stripped
                            } else {
                                tag
                            };
                            return Ok(version.to_string());
                        }
                    }
                }
            }
        }

        Err(anyhow!("Could not extract version from release info"))
    }
}
