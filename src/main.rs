use anyhow::Result;
use clap::{Arg, Command};
use std::io::{stdout, IsTerminal};
use std::path::{Path, PathBuf};
use tracing::{subscriber, Level};
use tracing_subscriber::fmt;
use yek::{find_config_file, load_config_file, parse_size_input, serialize_repo, YekConfig};

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
                .help("Count size in tokens instead of bytes")
                .action(clap::ArgAction::SetTrue),
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
        yek_config.max_size = Some(parse_size_input(size_str, matches.get_flag("tokens"))?);
    }

    yek_config.token_mode = matches.get_flag("tokens");

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

    // Print output directory if not streaming
    if !yek_config.stream {
        if let Some(dir) = &yek_config.output_dir {
            tracing::info!("Output directory: {}", dir.display());
        }
    }

    // Run serialize_repo for each directory
    for dir in directories {
        let path = Path::new(dir);

        // Make a per-directory clone of base config
        let mut config_for_this_dir = yek_config.clone();

        // Look up any local yek.toml
        if let Some(toml_path) = find_config_file(path) {
            if let Some(file_cfg) = load_config_file(&toml_path) {
                // Merge file_cfg into config_for_this_dir
                merge_config(&mut config_for_this_dir, &file_cfg);
            }
        }

        serialize_repo(path, Some(&config_for_this_dir))?;
    }

    Ok(())
}

/// Merge the fields of `other` into `dest`.
fn merge_config(dest: &mut YekConfig, other: &YekConfig) {
    // Merge ignore patterns
    dest.ignore_patterns
        .extend_from_slice(&other.ignore_patterns);
    // Merge priority rules
    dest.priority_rules.extend_from_slice(&other.priority_rules);
    // Merge binary extensions
    dest.binary_extensions
        .extend_from_slice(&other.binary_extensions);

    // Respect whichever max_size is more specific
    if dest.max_size.is_none() && other.max_size.is_some() {
        dest.max_size = other.max_size;
    }

    // token_mode: if `other` is true, set it
    if other.token_mode {
        dest.token_mode = true;
    }

    // If `other.output_dir` is set, we can choose to override or not. Usually the CLI
    // argument has higher precedence, so we only override if `dest.output_dir` is None:
    if dest.output_dir.is_none() && other.output_dir.is_some() {
        dest.output_dir = other.output_dir.clone();
    }

    // Similarly for stream
    if !dest.stream && other.stream {
        // only override if CLI didn't force an output dir
        if dest.output_dir.is_none() {
            dest.stream = true;
        }
    }
}
