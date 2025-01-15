use anyhow::Result;
use clap::{Arg, ArgAction, Command};
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use tracing::{debug, info, Level};
use tracing_subscriber::FmtSubscriber;
use yek::{find_config_file, load_config_file, serialize_repo};

fn main() -> Result<()> {
    let matches = Command::new("yek")
        .about("Serialize repository content for LLM context")
        .arg(
            Arg::new("path")
                .help("Path to repository")
                .default_value(".")
                .index(1),
        )
        .arg(
            Arg::new("max-size")
                .help("Maximum size in MB")
                .short('x')
                .long("max-size")
                .value_parser(clap::value_parser!(usize))
                .default_value("10"),
        )
        .arg(
            Arg::new("config")
                .help("Path to config file")
                .short('c')
                .long("config"),
        )
        .arg(
            Arg::new("output-dir")
                .help("Directory to write output files (overrides config file)")
                .short('o')
                .long("output-dir"),
        )
        .arg(
            Arg::new("tokens")
                .short('k')
                .long("tokens")
                .help("Count in tokens instead of bytes")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("path-prefix")
                .short('p')
                .long("path-prefix")
                .help("Only process files under this path prefix")
                .value_name("PREFIX"),
        )
        .arg(
            Arg::new("debug")
                .help("Enable debug logging")
                .short('v')
                .long("debug")
                .action(ArgAction::SetTrue),
        )
        .get_matches();

    // Initialize logging based on debug flag
    FmtSubscriber::builder()
        .with_max_level(if matches.get_flag("debug") {
            Level::DEBUG
        } else {
            Level::INFO
        })
        .with_target(false)
        .with_thread_ids(false)
        .with_thread_names(false)
        .with_file(false)
        .with_line_number(false)
        .with_level(true)
        .with_ansi(true)
        .with_timer(tracing_subscriber::fmt::time::LocalTime::new(
            time::format_description::parse("[hour]:[minute]:[second]").unwrap(),
        ))
        .compact()
        .init();

    debug!("Starting yek with debug logging enabled");

    let path = matches
        .get_one::<String>("path")
        .map(|s| s.as_str())
        .unwrap_or(".");
    let count_tokens = matches.get_flag("tokens");
    let max_size = matches
        .get_one::<usize>("max-size")
        .map(|s| if count_tokens { *s } else { s * 1024 * 1024 })
        .unwrap_or(10 * 1024 * 1024);
    let stream = !std::io::stdout().is_terminal();
    let output_dir = matches.get_one::<String>("output-dir").map(Path::new);
    let path_prefix = matches.get_one::<String>("path-prefix").map(|s| s.as_str());

    debug!("CLI Arguments:");
    debug!("  Repository path: {}", path);
    debug!("  Maximum size: {} bytes", max_size);
    debug!("  Stream mode: {}", stream);
    debug!("  Token counting mode: {}", count_tokens);
    debug!("  Output directory: {:?}", output_dir);

    let config_path = matches
        .get_one::<String>("config")
        .map(PathBuf::from)
        .or_else(|| find_config_file(Path::new(path)));

    let config = config_path.and_then(|p| load_config_file(&p));
    debug!("Configuration:");
    debug!("  Config file loaded: {}", config.is_some());
    if let Some(cfg) = &config {
        debug!("  Ignore patterns: {}", cfg.ignore_patterns.patterns.len());
        debug!("  Priority rules: {}", cfg.priority_rules.len());
        debug!("  Binary extensions: {}", cfg.binary_extensions.len());
        debug!("  Output directory: {:?}", cfg.output_dir);
    }

    if let Some(output_path) = serialize_repo(
        max_size,
        Some(Path::new(path)),
        count_tokens,
        stream,
        config,
        output_dir,
        path_prefix,
    )? {
        info!("Output written to: {}", output_path.display());
    }

    Ok(())
}
