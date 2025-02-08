use anyhow::{anyhow, Result};
use bytesize::ByteSize;
use clap::Parser;
use clap_config_file::ClapConfigFile;
use sha2::{Digest, Sha256};
use std::{fs, path::Path, str::FromStr, time::UNIX_EPOCH};

use crate::{
    defaults::{BINARY_FILE_EXTENSIONS, DEFAULT_IGNORE_PATTERNS, DEFAULT_OUTPUT_TEMPLATE},
    priority::PriorityRule,
};

#[derive(Parser, ClapConfigFile, Clone)]
#[command(
    allow_external_subcommands = true,
    version,
    about = "Yek repository serialization tool"
)]
#[config_file_name = "yek"]
#[config_file_formats = "toml,yaml,json"]
pub struct YekConfig {
    /// Input directories to process
    #[config_arg(positional)]
    pub input_dirs: Vec<String>,

    /// Max size per chunk. e.g. "10MB" or "128K" or when using token counting mode, "100" or "128K"
    #[config_arg(default_value = "10MB")]
    pub max_size: String,

    /// Use token mode instead of byte mode
    #[config_arg()]
    pub tokens: String,

    /// Enable JSON output
    #[config_arg()]
    #[arg(long)]
    pub json: bool,

    /// Enable debug output
    #[config_arg()]
    #[arg(long)]
    pub debug: bool,

    /// Output directory. If none is provided & stdout is a TTY, we pick a temp dir
    #[config_arg()]
    pub output_dir: Option<String>,

    /// Output template. Defaults to ">>>> FILE_PATH\nFILE_CONTENT"
    #[config_arg(default_value = DEFAULT_OUTPUT_TEMPLATE)]
    pub output_template: String,

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

    /// True if we should stream output to stdout (computed)
    pub stream: bool,

    /// True if we should count tokens, not bytes (computed)
    pub token_mode: bool,

    /// Final resolved output file path (only used if not streaming)
    pub output_file_full_path: Option<String>,

    /// Maximum depth to search for Git commit times
    #[config_arg(accept_from = "config_only", default_value = "100")]
    pub max_git_depth: i32,

    /// Capture any extra CLI arguments not recognized by YekConfig.
    #[clap(trailing_var_arg = true)]
    pub extra_args: Option<Vec<String>>,

    /// Version flag (computed). Default is false.
    #[arg(skip)]
    pub version: bool,
}

impl Default for YekConfig {
    fn default() -> Self {
        Self {
            input_dirs: Vec::new(),
            max_size: "10MB".to_string(),
            tokens: String::new(),
            json: false,
            debug: false,
            output_dir: None,
            output_template: DEFAULT_OUTPUT_TEMPLATE.to_string(),
            ignore_patterns: Vec::new(),
            unignore_patterns: Vec::new(),
            priority_rules: Vec::new(),
            binary_extensions: BINARY_FILE_EXTENSIONS
                .iter()
                .map(|s| s.to_string())
                .collect(),
            git_boost_max: Some(100),
            // computed fields
            stream: false,
            token_mode: false,
            output_file_full_path: None,
            max_git_depth: 100,
            extra_args: None,
            version: false,
        }
    }
}

impl YekConfig {
    pub fn extend_config_with_defaults(input_dirs: Vec<String>, output_dir: String) -> Self {
        YekConfig {
            input_dirs,
            output_dir: Some(output_dir),
            ..Default::default()
        }
    }
}

impl YekConfig {
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

        fs::create_dir_all(path)
            .map_err(|e| anyhow!("output_dir: cannot create '{}': {}", output_dir, e))?;

        Ok(output_dir)
    }

    /// Parse from CLI + config file, fill in computed fields, and validate.
    pub fn init_config() -> Self {
        // Use a fixed argument list if the special environment variable is set.
        let mut cfg = if std::env::var("YEK_CLI_TEST").is_ok() {
            YekConfig::parse_from(std::iter::once("yek"))
        } else {
            YekConfig::parse()
        };

        // Compute derived fields:
        cfg.token_mode = !cfg.tokens.is_empty();
        let force_tty = std::env::var("FORCE_TTY").is_ok();

        cfg.stream = !atty::is(atty::Stream::Stdout) && !force_tty;

        // Default input dirs to current dir if none:
        if cfg.input_dirs.is_empty() {
            cfg.input_dirs.push(".".to_string());
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

        // If no output_template is provided (or it is empty), set it to default.
        if cfg.output_template.trim().is_empty() {
            cfg.output_template = DEFAULT_OUTPUT_TEMPLATE.to_string();
        }

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

        // Validate the config.
        if let Err(e) = cfg.validate() {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }

        cfg
    }

    /// Compute a quick checksum for the *top-level listing* of each input dir.
    pub fn get_checksum(input_dirs: &[String]) -> String {
        let mut hasher = Sha256::new();
        for dir in input_dirs {
            let base_path = Path::new(dir);
            if !base_path.exists() {
                continue;
            }
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
        if !self.output_template.contains("FILE_PATH")
            || !self.output_template.contains("FILE_CONTENT")
        {
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
            let val = self.tokens[..self.tokens.len() - 1]
                .trim()
                .parse::<usize>()
                .map_err(|e| anyhow!("tokens: Invalid token size: {}", e))?;
            if val == 0 {
                return Err(anyhow!("tokens: cannot be 0"));
            }
        } else if !self.tokens.is_empty() {
            let val = self
                .tokens
                .parse::<usize>()
                .map_err(|e| anyhow!("tokens: Invalid token size: {}", e))?;
            if val == 0 {
                return Err(anyhow!("tokens: cannot be 0"));
            }
        }

        if !self.stream {
            self.ensure_output_dir()?;
        }

        for pattern in &self.ignore_patterns {
            glob::Pattern::new(pattern)
                .map_err(|e| anyhow!("ignore_patterns: Invalid pattern '{}': {}", pattern, e))?;
        }

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

        Ok(())
    }
}

impl YekConfig {
    /// Ensure that the output directory is valid and exists.
    pub fn ensure_output_dir_valid(&self) -> Result<String> {
        self.ensure_output_dir()
    }
}
