use anyhow::{anyhow, Result};
use clap_config_file::ClapConfigFile;
use fnmatch_regex;
use regex::Regex;
use std::{io::IsTerminal, path::Path};

use crate::{
    defaults::{default_ignore_patterns, BINARY_FILE_EXTENSIONS},
    priority::PriorityRule,
};

#[derive(Clone, Debug, Default, clap::ValueEnum, serde::Serialize, serde::Deserialize)]
pub enum ConfigFormat {
    #[default]
    Toml,
    Yaml,
    Json,
}

#[derive(ClapConfigFile)]
#[config_file_name = "yek"]
#[config_file_formats = "toml,yaml,json"]
pub struct YekConfig {
    /// Input directories to process
    #[config_arg(accept_from = "cli_only")]
    #[config_arg(value_parser = clap::value_parser!(Vec<String>))]
    #[config_arg(long = "input-dirs", required = true)]
    pub input_dirs: Vec<String>,

    /// Max size per chunk. e.g. "10MB" or "128K" or when using token counting mode, "100" or "128K"
    #[config_arg(long = "max-size", default_value = "10MB")]
    pub max_size: String,

    /// Use token mode instead of byte mode
    #[config_arg(long)]
    pub tokens: String,

    /// Enable debug output
    #[config_arg(long, default_value = "false")]
    pub debug: Option<bool>,

    /// Output directory. If none is provided & stdout is a TTY, we pick a temp dir
    #[config_arg(long = "output-dir")]
    pub output_dir: Option<String>,

    /// Ignore patterns
    #[config_arg(long = "ignore-patterns", multi_value_behavior = "extend")]
    pub ignore_patterns: Vec<String>,

    /// Priority rules
    #[config_arg(accept_from = "config_only")]
    pub priority_rules: Vec<PriorityRule>,

    /// Binary file extensions to ignore
    #[config_arg(accept_from = "config_only")]
    pub binary_extensions: Vec<String>,

    /// Stream output to stdout
    pub stream: bool,

    /// Use token mode instead of byte mode
    pub token_mode: bool,

    /// The full path to the output file
    pub output_file_full_path: String,

    /// The format of the config file. Defaults to "toml"
    pub config_file_format: ConfigFormat,

    /// Maximum additional boost from Git commit times (0..1000)
    #[config_arg(accept_from = "config_only")]
    pub git_boost_max: Option<i32>,
}

// Define a struct that is "complete" yet config with all fields being required
#[derive(Clone, serde::Serialize)]
pub struct FullYekConfig {
    pub input_dirs: Vec<String>,
    pub max_size: String,
    pub tokens: String,
    pub debug: bool,
    pub output_dir: String,
    pub ignore_patterns: Vec<String>,
    pub priority_rules: Vec<PriorityRule>,
    pub binary_extensions: Vec<String>,
    pub stream: bool,
    pub token_mode: bool,
    pub output_file_full_path: String,
    pub git_boost_max: i32,
}

impl YekConfig {
    /// Initialize the config from CLI arguments + optional `yek.toml`.
    pub fn init_config() -> FullYekConfig {
        let (defaults, _, _) = YekConfig::parse_info();

        let token_mode = !defaults.tokens.is_empty();
        let stream = !std::io::stdout().is_terminal();

        // Default the output dir to a temp dir if not provided
        let output_dir = if let Some(dir) = defaults.output_dir {
            dir
        } else {
            let temp_dir = std::env::temp_dir();
            let output_dir = temp_dir.join("yek-output");
            std::fs::create_dir_all(&output_dir).unwrap();
            output_dir.to_string_lossy().to_string()
        };

        // Merge user binary extensions with built-ins
        let binary_extensions = defaults
            .binary_extensions
            .into_iter()
            .chain(BINARY_FILE_EXTENSIONS.iter().map(|&s| s.to_string()))
            .collect();

        // Merge user ignore patterns with defaults
        let ignore_patterns = defaults
            .ignore_patterns
            .into_iter()
            .chain(default_ignore_patterns().into_iter().map(|p| p.to_string()))
            .collect();

        // TODO: make the output file name based on checksum of input dirs contents
        // and short circuit if the file already exists
        // Default the output file to a temp file in the output dir
        let output_file_full_path = Path::new(&output_dir)
            .join("yek-output.txt")
            .to_string_lossy()
            .to_string();

        let final_config = FullYekConfig {
            input_dirs: defaults.input_dirs,
            max_size: defaults.max_size,
            tokens: defaults.tokens,
            debug: defaults.debug.unwrap_or(false),
            output_dir,
            ignore_patterns,
            priority_rules: defaults.priority_rules,
            binary_extensions,
            stream,
            token_mode,
            output_file_full_path,
            git_boost_max: defaults.git_boost_max.unwrap_or(100),
        };

        // Validate the config
        validate_config(&final_config);

        final_config
    }
}

pub struct ConfigError {
    pub field: String,
    pub message: String,
}

pub fn validate_config(config: &FullYekConfig) -> Vec<ConfigError> {
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
        let regex_str = if pattern.starts_with('^') || pattern.ends_with('$') {
            pattern.to_string()
        } else {
            match fnmatch_regex::glob_to_regex(pattern) {
                Ok(r) => r.to_string(),
                Err(e) => {
                    errors.push(ConfigError {
                        field: "ignore_patterns".to_string(),
                        message: format!("Invalid pattern '{}': {}", pattern, e),
                    });
                    continue;
                }
            }
        };

        if let Err(e) = Regex::new(&regex_str) {
            errors.push(ConfigError {
                field: "ignore_patterns".to_string(),
                message: format!("Invalid pattern '{}': {}", pattern, e),
            });
        }
    }

    // Validate max_size
    if config.max_size == "0" {
        errors.push(ConfigError {
            field: "max_size".to_string(),
            message: "Max size cannot be 0".to_string(),
        });
    }

    // Validate output directory if specified
    let path = Path::new(&config.output_dir);
    if path.exists() && !path.is_dir() {
        errors.push(ConfigError {
            field: "output_dir".to_string(),
            message: format!(
                "Output path '{}' exists but is not a directory",
                config.output_dir
            ),
        });
    }

    if let Err(e) = std::fs::create_dir_all(path) {
        errors.push(ConfigError {
            field: "output_dir".to_string(),
            message: format!(
                "Cannot create output directory '{}': {}",
                config.output_dir, e
            ),
        });
    }

    errors
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
