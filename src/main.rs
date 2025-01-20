use anyhow::Result;
use clap::{Arg, Command};
use std::io::{stdout, IsTerminal};
use std::path::{Path, PathBuf};
use tracing::Level;
use tracing_subscriber::fmt;
use yek::{parse_size_input, serialize_repo, YekConfig};

fn main() -> Result<()> {
    let matches = Command::new("yek")
        .version(env!("CARGO_PKG_VERSION"))
        .about("A tool to serialize repository content")
        .arg(
            Arg::new("directories")
                .help("Directories to process")
                .num_args(0..)
                .default_value("."),
        )
        .arg(
            Arg::new("output-dir")
                .long("output-dir")
                .short('o')
                .help("Output directory for chunk files"),
        )
        .arg(
            Arg::new("max-size")
                .long("max-size")
                .help("Maximum size of each chunk in bytes or with K/M/G suffix"),
        )
        .arg(
            Arg::new("debug")
                .long("debug")
                .help("Enable debug output")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("tokens")
                .long("tokens")
                .help("Use token-based chunking instead of byte-based")
                .action(clap::ArgAction::SetTrue),
        )
        .get_matches();

    // Setup logging
    let level = if matches.get_flag("debug") {
        Level::DEBUG
    } else {
        Level::INFO
    };
    fmt()
        .with_max_level(level)
        .without_time()
        .with_target(false)
        .with_ansi(true)
        .init();

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

    // Run serialize_repo for each directory
    for dir in directories {
        serialize_repo(Path::new(dir), Some(&yek_config))?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_size_input_tokens() {
        assert_eq!(parse_size_input("100K", true).unwrap(), 100_000);
        assert_eq!(parse_size_input("100k", true).unwrap(), 100_000);
        assert_eq!(parse_size_input("0K", true).unwrap(), 0);
        assert_eq!(parse_size_input("1K", true).unwrap(), 1_000);
        assert_eq!(parse_size_input("1k", true).unwrap(), 1_000);

        // Plain numbers
        assert_eq!(parse_size_input("100", true).unwrap(), 100);
        assert_eq!(parse_size_input("1000", true).unwrap(), 1000);
        assert_eq!(parse_size_input("0", true).unwrap(), 0);

        // Invalid cases
        assert!(parse_size_input("K", true).is_err());
        assert!(parse_size_input("-1K", true).is_err());
        assert!(parse_size_input("-100", true).is_err());
        assert!(parse_size_input("100KB", true).is_err());
        assert!(parse_size_input("invalid", true).is_err());
        assert!(parse_size_input("", true).is_err());
        assert!(parse_size_input(" ", true).is_err());
        assert!(parse_size_input("100K100", true).is_err());
        assert!(parse_size_input("100.5K", true).is_err());

        // Whitespace handling
        assert_eq!(parse_size_input(" 100K ", true).unwrap(), 100_000);
        assert_eq!(parse_size_input("\t100k\n", true).unwrap(), 100_000);
        assert_eq!(parse_size_input(" 100 ", true).unwrap(), 100);
    }

    #[test]
    fn test_parse_size_input_bytes() {
        // KB
        assert_eq!(parse_size_input("100KB", false).unwrap(), 102_400);
        assert_eq!(parse_size_input("100kb", false).unwrap(), 102_400);
        assert_eq!(parse_size_input("0KB", false).unwrap(), 0);
        assert_eq!(parse_size_input("1KB", false).unwrap(), 1_024);

        // MB
        assert_eq!(parse_size_input("1MB", false).unwrap(), 1_048_576);
        assert_eq!(parse_size_input("1mb", false).unwrap(), 1_048_576);
        assert_eq!(parse_size_input("0MB", false).unwrap(), 0);

        // GB
        assert_eq!(parse_size_input("1GB", false).unwrap(), 1_073_741_824);
        assert_eq!(parse_size_input("1gb", false).unwrap(), 1_073_741_824);
        assert_eq!(parse_size_input("0GB", false).unwrap(), 0);

        // Plain bytes
        assert_eq!(parse_size_input("1024", false).unwrap(), 1024);
        assert_eq!(parse_size_input("0", false).unwrap(), 0);

        // Invalid cases
        assert!(parse_size_input("invalid", false).is_err());
        assert!(parse_size_input("", false).is_err());
        assert!(parse_size_input(" ", false).is_err());
        assert!(parse_size_input("-1KB", false).is_err());
        assert!(parse_size_input("-1024", false).is_err());
        assert!(parse_size_input("1.5KB", false).is_err());
        assert!(parse_size_input("1K", false).is_err()); // Must be KB
        assert!(parse_size_input("1M", false).is_err()); // Must be MB
        assert!(parse_size_input("1G", false).is_err()); // Must be GB

        // Whitespace handling
        assert_eq!(parse_size_input(" 100KB ", false).unwrap(), 102_400);
        assert_eq!(parse_size_input("\t100kb\n", false).unwrap(), 102_400);
        assert_eq!(parse_size_input(" 1024 ", false).unwrap(), 1024);
    }
}
