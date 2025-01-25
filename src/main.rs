use anyhow::{anyhow, Result};
use clap::Parser;
use std::fs;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use tracing::debug;
use yek::{model_manager, parse_size_input, process_directory, validate_config, YekConfig};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
#[command(after_help = "See https://github.com/mohsen-w-elsayed/yek for detailed documentation.")]
struct Args {
    /// Directories to process
    #[arg()]
    directories: Vec<PathBuf>,

    /// Path to custom config file
    #[arg(short, long)]
    config: Option<PathBuf>,

    /// Maximum output size (supports K/KB/M/MB suffixes)
    #[arg(long, value_name = "SIZE")]
    max_size: Option<String>,

    #[arg(long, value_name = "MODEL")]
    #[arg(num_args = 0..=1, require_equals = true, default_missing_value = "openai")]
    #[arg(value_parser = ["openai", "claude", "mistral", "mixtral", "deepseek", "llama", "codellama"])]
    #[arg(help = "Count size in tokens using specified model family (default: openai)\nSUPPORTED MODELS: openai, claude, mistral, mixtral, deepseek, llama, codellama")]
    tokens: Option<String>,

    /// Output directory for generated files
    #[arg(long, short, value_name = "DIR")]
    output_dir: Option<PathBuf>,

    /// Enable debug output
    #[arg(long)]
    debug: bool,
}

const SUPPORTED_MODELS: &str = "openai, claude, mistral, mixtral, deepseek, llama, codellama";

impl Args {
    fn model_family(&self) -> Option<&str> {
        self.tokens.as_deref().filter(|s| !s.is_empty())
    }
}

fn load_config(path: &Path) -> Result<YekConfig> {
    let contents = std::fs::read_to_string(path)?;
    toml::from_str(&contents).map_err(|e| anyhow!("Failed to parse config: {}", e))
}

fn main() -> Result<()> {
    let args = Args::parse();
    println!("args: {:?}", args);
    let mut config = YekConfig::default();

    // Load config from file if specified
    if let Some(config_path) = args.config.clone() {
        config = load_config(&config_path)?;
    }

    // Merge command-line arguments into config
    if let Some(size_str) = &args.max_size {
        config.max_size = Some(parse_size_input(size_str, config.token_mode)?);
    }

    if let Some(model) = args.model_family() {
        if !model_manager::SUPPORTED_MODEL_FAMILIES.contains(&model) {
            return Err(anyhow!(
                "Unsupported model family '{}'. Supported: {}",
                model,
                SUPPORTED_MODELS
            ));
        }
        config.token_mode = true;
        config.tokenizer_model = Some(model.to_string());
    } else if config.token_mode {
        let model = config.tokenizer_model.as_deref().unwrap_or("openai");
        tracing::debug!("Token mode enabled via config with model: {}", model);
    }

    // Validate tokenizer model from config file
    if let Some(model) = &config.tokenizer_model {
        if !model_manager::SUPPORTED_MODEL_FAMILIES.contains(&model.as_str()) {
            return Err(anyhow!(
                "Unsupported tokenizer model '{}' in config. Supported: {}",
                model,
                SUPPORTED_MODELS
            ));
        }
    }

    if let Some(output_dir) = args.output_dir {
        config.output_dir = Some(output_dir);
    }

    // Use current directory if no directories provided
    let directories = if args.directories.is_empty() {
        vec![PathBuf::from(".")]
    } else {
        args.directories
    };

    // Handle stream detection
    config.stream = !std::io::stdout().is_terminal();
    // Force stream=false if output_dir is specified
    if config.output_dir.is_some() {
        config.stream = false;
    }

    if args.debug {
        use tracing_subscriber::{fmt, EnvFilter};
        use std::fs::File;
        let filter = EnvFilter::builder().with_default_directive("yek=debug".parse().unwrap()).from_env_lossy();
        let fmt = fmt().with_env_filter(filter).with_ansi(false);
        if let Ok(path) = std::env::var("YEK_DEBUG_OUTPUT") {
            let file = File::create(path)?;
            fmt.with_writer(file).init();
        } else {
            fmt.with_writer(std::io::stderr).init();
        }
    }

    // Process directories
    for path in directories {
        let mut config_for_this_dir = config.clone();

        // Load directory-specific config if it exists
        let dir_config_path = path.join("yek.toml");
        if dir_config_path.exists() {
            let dir_config = load_config(&dir_config_path)?;
            // Validate tokenizer model from directory config
            if let Some(model) = &dir_config.tokenizer_model {
                if !model_manager::SUPPORTED_MODEL_FAMILIES.contains(&model.as_str()) {
                    return Err(anyhow!(
                        "Unsupported tokenizer model '{}' in directory config. Supported: {}",
                        model,
                        SUPPORTED_MODELS
                    ));
                }
            }
            config_for_this_dir.merge(&dir_config);
        }

        // Resolve output directory relative to current directory being processed
        if let Some(output_dir) = config_for_this_dir.output_dir.take() {
            let resolved_output = path.join(output_dir);
            debug!("Resolving output dir {:?} -> {:?}", path, resolved_output);
            config_for_this_dir.output_dir = Some(resolved_output);
        }

        // Ensure output directory exists even if no content
        if let Some(out_dir) = &config_for_this_dir.output_dir {
            fs::create_dir_all(out_dir)?;
        }

        // Validate final merged config
        let errors = validate_config(&config_for_this_dir);
        if !errors.is_empty() {
            for error in errors {
                eprintln!("Config error: {}", error);
            }
            return Err(anyhow!("Invalid configuration"));
        }

        // Process the directory
        process_directory(&path, &config_for_this_dir)?;
    }

    Ok(())
}
