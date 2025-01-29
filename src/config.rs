use anyhow::{anyhow, Result};
use bytesize::ByteSize;
use clap_config_file::ClapConfigFile;
use sha2::{Digest, Sha256};
use tracing::debug;
use std::{io::IsTerminal, path::Path, str::FromStr};

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
#[config_file_name(name = "yek")]
#[config_file_formats(format = "toml,yaml,json")]
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
    pub json: bool,

    /// Enable debug output
    #[config_arg()]
    pub debug: bool,

    /// Output directory. If none is provided & stdout is a TTY, we pick a temp dir
    #[config_arg()]
    pub output_dir: Option<String>,

    /// Output template. Defaults to ">>>> FILE_PATH\nFILE_CONTENT"
    #[config_arg(default_value = ">>>> FILE_PATH\nFILE_CONTENT")]
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
}

#[derive(Clone, serde::Serialize)]
pub struct FullYekConfig {
    #[serde(flatten)]
    pub base: YekConfig,
    /// Stream output to stdout
    pub stream: bool,
    /// Use token mode instead of byte mode
    pub token_mode: bool,
    /// The full path to the output file
    pub output_file_full_path: String,
}

impl std::ops::Deref for FullYekConfig {
    type Target = YekConfig;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl std::ops::DerefMut for FullYekConfig {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl FullYekConfig {
    pub fn extend_config_with_defaults(
        input_dirs: Vec<String>,
        output_dir: String,
    ) -> FullYekConfig {
        let token_mode = false;
        let checksum = get_checksum(&input_dirs);
        let extension = if token_mode { "tok" } else { "txt" };
        let output_file_full_path = Path::new(&output_dir)
            .join(format!("yek-output-{}.{}", checksum, extension))
            .to_string_lossy()
            .to_string();

        FullYekConfig {
            base: YekConfig {
                input_dirs,
                output_dir: Some(output_dir),
                max_size: "10MB".to_string(),
                tokens: String::new(),
                json: false,
                debug: false,
                output_template: DEFAULT_OUTPUT_TEMPLATE.to_string(),
                ignore_patterns: Vec::new(),
                unignore_patterns: Vec::new(),
                priority_rules: Vec::new(),
                binary_extensions: Vec::new(),
                git_boost_max: Some(100),
            },
            stream: false,
            token_mode: false,
            output_file_full_path,
        }
    }
}

fn get_checksum(input_dirs: &Vec<String>) -> String {
    // Generate a checksum of the input directories' contents
    let mut hasher = Sha256::new();
    for dir in input_dirs {
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
    format!("{:x}", result)
}

impl YekConfig {
    /// Initialize the config from CLI arguments + optional `yek.toml`.
    pub fn init_config() -> FullYekConfig {
        let (mut config, config_file_path, _) = YekConfig::parse_info();

        // if config.debug {
            debug!("Config file path: {:?}", config_file_path);
        // }

        // Compute values that are computed
        let token_mode = !config.tokens.is_empty();
        let stream = !std::io::stdout().is_terminal();

        // Default input dirs to current dir if not provided
        if config.input_dirs.is_empty() {
            config.input_dirs.push(".".to_string());
        }

        // Extend binary extensions with defaults
        config
            .binary_extensions
            .extend(BINARY_FILE_EXTENSIONS.iter().map(|s| s.to_string()));

        // Extend ignore patterns with defaults
        config
            .ignore_patterns
            .extend(DEFAULT_IGNORE_PATTERNS.iter().map(|s| s.to_string()));

        // Apply unignore patterns to ignore patterns
        config.ignore_patterns.extend(config.unignore_patterns.iter().map(|s| 
            // Change the glob to a negative glob by adding ! to the beginning
            format!("!{}", s)));

        // Default the output dir to a temp dir if not provided
        let output_dir = if let Some(dir) = config.output_dir {
            dir
        } else {
            let temp_dir = std::env::temp_dir();
            let output_dir = temp_dir.join("yek-output");
            std::fs::create_dir_all(&output_dir).unwrap();
            output_dir.to_string_lossy().to_string()
        };

        let checksum = get_checksum(&config.input_dirs);
        let extension = if config.json { "json" } else { "txt" };
        // Make the output file name based on checksum of input dirs contents
        let output_file_full_path = Path::new(&output_dir)
            .join(format!("yek-output-{}.{}", checksum, extension))
            .to_string_lossy()
            .to_string();

        let final_config = FullYekConfig {
            base: YekConfig {
                input_dirs: config.input_dirs.clone(),
                output_dir: Some(output_dir),
                max_size: config.max_size.clone(),
                tokens: config.tokens.clone(),
                json: config.json,
                debug: config.debug,
                output_template: config.output_template.clone(),
                ignore_patterns: config.ignore_patterns.clone(),
                unignore_patterns: config.unignore_patterns.clone(),
                priority_rules: config.priority_rules.clone(),
                binary_extensions: config.binary_extensions.clone(),
                git_boost_max: config.git_boost_max,
            },
            stream,
            token_mode,
            output_file_full_path,
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
    // Validate output template
    if !config.base.output_template.contains("FILE_PATH")
        || !config.base.output_template.contains("FILE_CONTENT")
    {
        return Err(anyhow!(
            "output_template: Output template must contain FILE_PATH and FILE_CONTENT"
        ));
    }

    // Validate max_size
    if config.base.max_size == "0" {
        return Err(anyhow!("max_size: Max size cannot be 0"));
    }

    if !config.token_mode {
        ByteSize::from_str(&config.base.max_size)
            .map_err(|e| anyhow!("max_size: Invalid size format: {}", e))?;
    } else if config.base.tokens.to_lowercase().ends_with('k') {
        let val = config.base.tokens[..config.base.tokens.len() - 1]
            .trim()
            .parse::<usize>()
            .map_err(|e| anyhow!("tokens: Invalid token size: {}", e))?;
        if val == 0 {
            return Err(anyhow!("tokens: Token size cannot be 0"));
        }
    } else {
        let val = config
            .base
            .tokens
            .parse::<usize>()
            .map_err(|e| anyhow!("tokens: Invalid token size: {}", e))?;
        if val == 0 {
            return Err(anyhow!("tokens: Token size cannot be 0"));
        }
    }

    // Validate output directory if specified
    let output_dir = config.base.output_dir.as_ref()
        .ok_or_else(|| anyhow!("output_dir: Output directory must be specified"))?;
    
    let path = Path::new(output_dir);
    if path.exists() && !path.is_dir() {
        return Err(anyhow!(
            "output_dir: Output path '{}' exists but is not a directory",
            output_dir
        ));
    }

    if let Err(e) = std::fs::create_dir_all(path) {
        return Err(anyhow!(
            "output_dir: Cannot create output directory '{}': {}",
            output_dir,
            e
        ));
    }

    Ok(())
}
