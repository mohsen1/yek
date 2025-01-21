use anyhow::Result;
use clap::{Arg, Command};
use std::io::{stdout, IsTerminal};
use std::path::{Path, PathBuf};
use tracing::{debug, subscriber, Level};
use tracing_subscriber::fmt;
use yek::{
    find_config_file, load_config_file, merge_config, parse_size_input, serialize_repo,
    validate_config, YekConfig, SUPPORTED_MODELS,
};

fn main() -> Result<()> {
    let matches = Command::new("yek")
        .about("Repository content chunker and serializer for LLM consumption")
        .arg(
            Arg::new("directories")
                .help("Directories to process")
                .num_args(0..)
                .default_value("."),
        )
        .arg(
            Arg::new("max-size")
                .long("max-size")
                .help("Maximum size per chunk (e.g. '10MB', '128KB', '1GB')")
                .default_value("10MB"),
        )
        .arg(
            Arg::new("tokens")
                .long("tokens")
                .help(format!("Count size in tokens instead of bytes using specified model (supported models: {})", SUPPORTED_MODELS.join(", ")))
                .value_name("model")
                .action(clap::ArgAction::Set)
                .num_args(0..=1)
                .default_missing_value("gpt-4")
                .value_parser(clap::builder::NonEmptyStringValueParser::new())
                .allow_hyphen_values(true)
                .required(false)
                .hide_default_value(true),
        )
        .arg(
            Arg::new("debug")
                .long("debug")
                .help("Enable debug output")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("output-dir")
                .long("output-dir")
                .help("Output directory for chunks"),
        )
        .get_matches();

    // Setup logging
    let level = if matches.get_flag("debug") {
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

    // Gather directories
    let directories: Vec<&str> = matches
        .get_many::<String>("directories")
        .unwrap()
        .map(|s| s.as_str())
        .collect();

    // Gather config
    let mut yek_config = YekConfig::default();

    // Possibly parse max size
    if let Some(size_str) = matches.get_one::<String>("max-size") {
        yek_config.max_size = Some(parse_size_input(size_str, matches.contains_id("tokens"))?);
    }

    // Handle token mode and model
    if matches.contains_id("tokens") {
        yek_config.token_mode = true;
        // Get model from argument or use default
        let model = matches
            .get_one::<String>("tokens")
            .map(|s| s.to_string())
            .unwrap_or_else(|| "gpt-4".to_string());

        // Validate model early
        if !SUPPORTED_MODELS.contains(&model.as_str()) {
            eprintln!(
                "Error: Unsupported model '{}'. Supported models: {}",
                model,
                SUPPORTED_MODELS.join(", ")
            );
            std::process::exit(1);
        }

        yek_config.tokenizer_model = Some(model);
        debug!(
            "Token mode enabled with model: {:?}",
            yek_config.tokenizer_model
        );
    }

    // Are we writing chunk files or streaming?
    // If --output-dir is given, we always write to that directory.
    // Otherwise, if stdout is not a TTY, we stream. If it *is* a TTY, create a temp dir.
    if let Some(out_dir) = matches.get_one::<String>("output-dir") {
        yek_config.output_dir = Some(PathBuf::from(out_dir));
    } else {
        let stdout_is_tty = stdout().is_terminal();
        if stdout_is_tty {
            // Write chunk files to a temporary directory
            let tmp = std::env::temp_dir().join("yek-serialize");
            yek_config.output_dir = Some(tmp);
        } else {
            // Stream to stdout
            yek_config.stream = true;
        }
    }

    // Run serialize_repo for each directory
    for dir in directories {
        let path = Path::new(dir);

        // Make a per-directory clone of base config
        let mut config_for_this_dir = yek_config.clone();

        // Load config file if it exists
        if let Some(config_path) = find_config_file(path) {
            debug!("Found config file: {}", config_path.display());
            if let Some(file_config) = load_config_file(&config_path) {
                merge_config(&mut config_for_this_dir, &file_config);
            }
        }

        // Validate config
        let errors = validate_config(&config_for_this_dir);
        if !errors.is_empty() {
            for error in errors {
                eprintln!("Error in {}: {}", error.field, error.message);
            }
            return Err(anyhow::anyhow!("Invalid configuration"));
        }

        // Run serialize_repo
        serialize_repo(path, Some(&config_for_this_dir))?;
    }

    Ok(())
}
