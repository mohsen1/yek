use anyhow::Result;
use byte_unit::Byte;
use clap::{Arg, ArgAction, Command};
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use tracing::{info, Level};
use tracing_subscriber::fmt;
use yek::{find_config_file, load_config_file, serialize_repo};

fn parse_size_input(input: &str, is_tokens: bool) -> std::result::Result<usize, String> {
    if is_tokens {
        // Handle token count with K suffix
        let input = input.trim();
        if input.to_uppercase().ends_with('K') {
            let num = input[..input.len() - 1]
                .parse::<usize>()
                .map_err(|e| format!("Invalid token count: {}", e))?;
            Ok(num * 1000)
        } else {
            input
                .parse::<usize>()
                .map_err(|e| format!("Invalid token count: {}", e))
        }
    } else {
        Byte::from_str(input)
            .map(|b| b.get_bytes() as usize)
            .map_err(|e| e.to_string())
    }
}

fn main() -> Result<()> {
    let matches = Command::new("yek")
        .version(env!("CARGO_PKG_VERSION"))
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
                .help("Maximum size per chunk (e.g. '10MB', '128KB', '1GB' or '100K' tokens when --tokens is used)")
                .default_value("10MB"),
        )
        .arg(
            Arg::new("tokens")
                .long("tokens")
                .help("Count size in tokens instead of bytes")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("debug")
                .long("debug")
                .help("Enable debug output")
                .action(ArgAction::SetTrue),
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
    fmt()
        .with_max_level(level)
        .with_target(false)
        .with_file(false)
        .with_line_number(false)
        .with_thread_ids(false)
        .with_thread_names(false)
        .with_level(false)
        .with_ansi(true)
        .without_time()
        .init();

    // Parse max size
    let max_size_str = matches.get_one::<String>("max-size").unwrap();
    let max_size = parse_size_input(max_size_str, matches.get_flag("tokens"))
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    // Get directories to process
    let directories: Vec<PathBuf> = matches
        .get_many::<String>("directories")
        .unwrap()
        .map(|s| Path::new(s).to_path_buf())
        .collect();

    // Get output directory from command line or config
    let output_dir = matches
        .get_one::<String>("output-dir")
        .map(|s| Path::new(s).to_path_buf());

    // Check if we're in stream mode (piped output)
    let stream = output_dir.is_none() && !std::io::stdout().is_terminal();

    for dir in directories {
        // Find config file for each directory
        let config = find_config_file(&dir).and_then(|p| load_config_file(&p));

        if let Some(output_path) = serialize_repo(
            max_size,
            Some(&dir),
            stream,
            matches.get_flag("tokens"),
            config,
            output_dir.as_deref(),
            None,
        )? {
            info!("Output written to {}", output_path.display());
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_size_input_bytes() {
        // Using byte_unit::Byte to calculate expected values
        assert_eq!(
            parse_size_input("10MB", false).unwrap(),
            Byte::from_str("10MB").unwrap().get_bytes() as usize
        );
        assert_eq!(
            parse_size_input("128KB", false).unwrap(),
            Byte::from_str("128KB").unwrap().get_bytes() as usize
        );
        assert_eq!(
            parse_size_input("1GB", false).unwrap(),
            Byte::from_str("1GB").unwrap().get_bytes() as usize
        );
        assert!(parse_size_input("invalid", false).is_err());
    }

    #[test]
    fn test_parse_size_input_tokens() {
        // Test K suffix variations
        assert_eq!(parse_size_input("100K", true).unwrap(), 100_000);
        assert_eq!(parse_size_input("100k", true).unwrap(), 100_000);
        assert_eq!(parse_size_input("0K", true).unwrap(), 0);
        assert_eq!(parse_size_input("1K", true).unwrap(), 1_000);
        assert_eq!(parse_size_input("1k", true).unwrap(), 1_000);

        // Test without K suffix
        assert_eq!(parse_size_input("100", true).unwrap(), 100);
        assert_eq!(parse_size_input("1000", true).unwrap(), 1000);
        assert_eq!(parse_size_input("0", true).unwrap(), 0);

        // Test invalid inputs
        assert!(parse_size_input("K", true).is_err());
        assert!(parse_size_input("-1K", true).is_err());
        assert!(parse_size_input("-100", true).is_err());
        assert!(parse_size_input("100KB", true).is_err());
        assert!(parse_size_input("invalid", true).is_err());
        assert!(parse_size_input("", true).is_err());
        assert!(parse_size_input(" ", true).is_err());
        assert!(parse_size_input("100K100", true).is_err());
        assert!(parse_size_input("100.5K", true).is_err());
    }

    #[test]
    fn test_parse_size_input_whitespace() {
        // Test whitespace handling
        assert_eq!(parse_size_input(" 100K ", true).unwrap(), 100_000);
        assert_eq!(parse_size_input("\t100k\n", true).unwrap(), 100_000);
        assert_eq!(parse_size_input(" 100 ", true).unwrap(), 100);
    }
}
