use anyhow::{anyhow, Result};
use clap::Parser;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use tracing::{subscriber, Level};
use tracing_subscriber::fmt;
use yek::{
    find_config_file, load_config_file, parse_size_input, serialize_repo, validate_config,
    YekConfig,
};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
#[command(after_help = "See https://github.com/mohsen-w-elsayed/yek for detailed documentation.")]
struct Args {
    /// Directories to process
    #[arg()]
    directories: Vec<PathBuf>,

    /// Path to custom config file
    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,

    /// Maximum output size (supports K/KB/M/MB suffixes)
    #[arg(long, value_name = "SIZE")]
    max_size: Option<String>,

    #[arg(long, value_name = "MODEL")]
    #[arg(num_args = 0..=1, require_equals = true, default_missing_value = "openai")]
    #[arg(value_parser = ["openai", "claude", "mistral", "mixtral", "deepseek", "llama", "codellama"])]
    #[arg(
        help = "Count size in tokens using specified model family (default: openai)\nSUPPORTED MODELS: openai, claude, mistral, mixtral, deepseek, llama, codellama"
    )]
    tokens: Option<String>,

    /// Output directory for generated files
    #[arg(long, short, value_name = "DIR")]
    output_dir: Option<PathBuf>,

    /// Enable debug output
    #[arg(long)]
    debug: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Setup logging
    let level = if args.debug {
        Level::DEBUG
    } else {
        Level::INFO
    };

    // Configure logging output
    if let Ok(debug_output) = std::env::var("YEK_DEBUG_OUTPUT") {
        let file = std::fs::File::create(debug_output)?;
        let subscriber = fmt()
            .with_max_level(level)
            .with_writer(file)
            .without_time()
            .with_target(false)
            .with_ansi(false)
            .finish();
        subscriber::set_global_default(subscriber)?;
    } else {
        fmt()
            .with_max_level(level)
            .without_time()
            .with_target(false)
            .with_ansi(true)
            .init();
    }

    // Load and merge configurations
    let mut config = YekConfig::default();

    // Load config from file if specified
    let config_path = args
        .config
        .clone()
        .or_else(|| find_config_file(Path::new(".")));
    if let Some(config_path) = config_path {
        if config_path.exists() {
            let file_config = load_config_file(&config_path);
            match file_config {
                Some(file_config) => {
                    config.merge(&file_config);
                }
                None => {
                    return Err(anyhow!(
                        "Failed to load config from: {}",
                        config_path.display()
                    ));
                }
            }
        }
    }

    // Apply command-line arguments
    if let Some(size_str) = args.max_size {
        config.max_size = Some(parse_size_input(&size_str, config.token_mode)?);
    }

    if let Some(model) = args.tokens {
        config.token_mode = true;
        config.tokenizer_model = Some(model);
    }

    if let Some(output_dir) = &args.output_dir {
        config.output_dir = Some(output_dir.clone());
    }

    // Determine if we should stream based on output_dir and stdout
    config.stream = if config.output_dir.is_some() {
        // Output directory is specified, don't stream
        false
    } else {
        // No output directory, check if we're piping!
        std::io::stdout().is_terminal()
    };

    // Validate the merged configuration
    let validation_errors = validate_config(&config);
    if !validation_errors.is_empty() {
        for error in validation_errors {
            eprintln!("Configuration error: {}", error);
        }
        return Err(anyhow!("Invalid configuration"));
    }

    // Use specified directories or default to current directory
    let directories = if args.directories.is_empty() {
        vec![PathBuf::from(".")]
    } else {
        args.directories
    };

    // Run serialization for each directory
    for dir in directories {
        serialize_repo(&dir, Some(&config))?;
    }

    Ok(())
}
