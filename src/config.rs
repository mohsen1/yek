use anyhow::{anyhow, Result};
use bytesize::ByteSize;
use clap_config_file::ClapConfigFile;
use fnmatch_regex;
use regex::Regex;
use sha2::{Digest, Sha256};
use std::{io::IsTerminal, path::Path, str::FromStr};

use crate::priority::PriorityRule;

#[derive(Clone, Debug, Default, clap::ValueEnum, serde::Serialize, serde::Deserialize)]
pub enum ConfigFormat {
    #[default]
    Toml,
    Yaml,
    Json,
}

#[derive(ClapConfigFile)]
#[config_file_name(name = "yek.toml")]
#[config_file_formats(format = "toml")]
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

    /// Enable debug output
    #[config_arg()]
    pub debug: bool,

    /// Output directory. If none is provided & stdout is a TTY, we pick a temp dir
    #[config_arg()]
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
        let (mut config, _, _) = YekConfig::parse_info();

        let token_mode = config.token_mode;
        let stream = !std::io::stdout().is_terminal() || config.stream;

        // Default input dirs to current dir if not provided
        if config.input_dirs.is_empty() {
            config.input_dirs.push(".".to_string());
        }

        // Default the output dir to a temp dir if not provided
        let output_dir = if let Some(dir) = config.output_dir {
            dir
        } else {
            let temp_dir = std::env::temp_dir();
            let output_dir = temp_dir.join("yek-output");
            std::fs::create_dir_all(&output_dir).unwrap();
            output_dir.to_string_lossy().to_string()
        };

        // Generate a checksum of the input directories' contents
        let mut hasher = Sha256::new();
        for dir in &config.input_dirs {
            let path = Path::new(dir);
            if path.exists() {
                let mut paths = std::fs::read_dir(path)
                    .unwrap()
                    .map(|res| res.map(|e| e.path()))
                    .collect::<Result<Vec<_>, std::io::Error>>()
                    .unwrap();
                paths.sort();
                for file_path in paths {
                    if file_path.is_file() {
                        if let Ok(contents) = std::fs::read(file_path) {
                            hasher.update(contents);
                        }
                    }
                }
            }
        }
        let result = hasher.finalize();
        let checksum = format!("{:x}", result);

        // Make the output file name based on checksum of input dirs contents
        let output_file_full_path = Path::new(&output_dir)
            .join(format!("yek-output-{}.txt", checksum))
            .to_string_lossy()
            .to_string();

        let final_config = FullYekConfig {
            input_dirs: config.input_dirs,
            max_size: config.max_size,
            tokens: config.tokens,
            debug: config.debug,
            output_dir,
            ignore_patterns: config.ignore_patterns,
            // TODO: clap-config-file should support this
            // if a field is only in the config file, it should be allowed to be a
            // custom struct
            priority_rules: vec![],
            binary_extensions: config.binary_extensions,
            stream,
            token_mode,
            output_file_full_path,
            git_boost_max: config.git_boost_max.unwrap_or(100),
        };

        // Validate the config
        if let Err(e) = validate_config(&final_config) {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }

        final_config
    }
}

pub struct ConfigError {
    pub field: String,
    pub message: String,
}

pub fn validate_config(config: &FullYekConfig) -> Result<()> {
    // Validate priority rules
    for rule in &config.priority_rules {
        if rule.score < 0 || rule.score > 1000 {
            return Err(anyhow!(
                "priority_rules: Priority score {} must be between 0 and 1000",
                rule.score
            ));
        }
        if rule.pattern.is_empty() {
            return Err(anyhow!("priority_rules: Priority rule must have a pattern"));
        }
        // Validate regex pattern
        if let Err(e) = Regex::new(&rule.pattern) {
            return Err(anyhow!(
                "priority_rules: Invalid regex pattern '{}': {}",
                rule.pattern,
                e
            ));
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
                    return Err(anyhow!(
                        "ignore_patterns: Invalid pattern '{}': {}",
                        pattern,
                        e
                    ));
                }
            }
        };

        if let Err(e) = Regex::new(&regex_str) {
            return Err(anyhow!(
                "ignore_patterns: Invalid pattern '{}': {}",
                pattern,
                e
            ));
        }
    }

    // Validate max_size
    if config.max_size == "0" {
        return Err(anyhow!("max_size: Max size cannot be 0"));
    }

    if !config.token_mode {
        ByteSize::from_str(&config.max_size)
            .map_err(|e| anyhow!("max_size: Invalid size format: {}", e))?;
    } else if config.tokens.to_lowercase().ends_with('k') {
        let val = config.tokens[..config.tokens.len() - 1]
            .trim()
            .parse::<usize>()
            .map_err(|e| anyhow!("tokens: Invalid token size: {}", e))?;
        if val == 0 {
            return Err(anyhow!("tokens: Token size cannot be 0"));
        }
    } else {
        let val = config
            .tokens
            .parse::<usize>()
            .map_err(|e| anyhow!("tokens: Invalid token size: {}", e))?;
        if val == 0 {
            return Err(anyhow!("tokens: Token size cannot be 0"));
        }
    }

    // Validate output directory if specified
    let path = Path::new(&config.output_dir);
    if path.exists() && !path.is_dir() {
        return Err(anyhow!(
            "output_dir: Output path '{}' exists but is not a directory",
            config.output_dir
        ));
    }

    if let Err(e) = std::fs::create_dir_all(path) {
        return Err(anyhow!(
            "output_dir: Cannot create output directory '{}': {}",
            config.output_dir,
            e
        ));
    }

    Ok(())
}
