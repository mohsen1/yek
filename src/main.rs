use anyhow::Result;
use clap::{Arg, Command};
use std::io::{stdout, IsTerminal};
use std::path::{Path, PathBuf};
use tracing::{debug, subscriber, Level};
use tracing_subscriber::fmt;
use yek::{
    find_config_file, load_config_file, merge_config, model_manager, parse_size_input,
    serialize_repo, validate_config, YekConfig,
};

fn main() -> Result<()> {
    let matches = Command::new("yek")
        .about("Repository content serializer for LLM consumption")
        .after_help(format!(
            "SUPPORTED MODELS:\n\
            Use with --tokens=MODEL\n\
            Available models:\n\
            {}\n\
            Each model family supports different token counting methods.",
            model_manager::SUPPORTED_MODEL_FAMILIES
                .iter()
                .map(|m| format!("  {} - {} models", m, m))
                .collect::<Vec<_>>()
                .join("\n")
        ))
        .arg(
            Arg::new("directories")
                .help("Directories to process")
                .num_args(0..)
                .default_value("."),
        )
        .arg(
            Arg::new("max-size")
                .long("max-size")
                .help("Maximum size of output file")
                .required(false),
        )
        .arg(
            Arg::new("tokens")
                .long("tokens")
                .help(format!(
                    "Count size in tokens using specified model family.\n\
                    Options: {}",
                    model_manager::SUPPORTED_MODEL_FAMILIES.join(", ")
                ))
                .value_name("MODEL_FAMILY")
                .num_args(0..=1)
                .value_parser(move |s: &str| {
                    if s.is_empty() {
                        Ok(String::new()) // Empty string indicates no model family specified
                    } else if model_manager::SUPPORTED_MODEL_FAMILIES.contains(&s) {
                        Ok(s.to_string())
                    } else {
                        Err(format!(
                            "Unsupported model family '{}'. Use --help to see supported model families.",
                            s
                        ))
                    }
                })
                .required(false),
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
                .help("Output directory for the output file"),
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
    let mut yek_config = YekConfig {
        token_mode: matches.contains_id("tokens"),
        tokenizer_model: if matches.contains_id("tokens") {
            matches.get_one::<String>("tokens").and_then(|model| {
                if !model.is_empty() {
                    Some(model.to_string())
                } else {
                    None
                }
            })
        } else {
            None
        },
        ..Default::default()
    };

    if yek_config.token_mode {
        debug!(
            "Token mode enabled{}",
            yek_config.tokenizer_model.as_ref().map_or_else(
                || " with default model: openai".to_string(),
                |m| format!(" with model: {}", m)
            )
        );
    }

    // Only parse max_size if provided as argument
    if let Some(size_str) = matches.get_one::<String>("max-size") {
        yek_config.max_size = Some(parse_size_input(size_str, yek_config.token_mode)?);
    }

    // Store command line output dir if specified
    if let Some(out_dir) = matches.get_one::<String>("output-dir") {
        yek_config.output_dir = Some(PathBuf::from(out_dir));
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

        // Handle output directory determination AFTER config merge
        let stdout_is_tty = stdout().is_terminal();

        // Handle streaming logic
        if !stdout_is_tty {
            // Force streaming when stdout is piped
            config_for_this_dir.stream = true;
            config_for_this_dir.output_dir = None;
        } else {
            // In interactive mode, set default output dir if none specified and not streaming
            if config_for_this_dir.output_dir.is_none() {
                let output_path = PathBuf::from("repo-serialized");
                if !output_path.exists() {
                    std::fs::create_dir_all(&output_path)?;
                }
                config_for_this_dir.output_dir = Some(output_path);
            }
            std::fs::create_dir_all(config_for_this_dir.output_dir.as_ref().unwrap())?;
        }

        // Validate final merged config
        let errors = validate_config(&config_for_this_dir);
        if !errors.is_empty() {
            for error in errors {
                eprintln!("Error in {}: {}", error.field, error.message);
            }
            eprintln!("Error: Invalid configuration");
            std::process::exit(1);
        }

        // Run serialize_repo
        serialize_repo(path, Some(&config_for_this_dir))?;
    }

    Ok(())
}
