use anyhow::Result;
use bytesize::ByteSize;
use rayon::join;
use std::path::Path;
use tracing::{debug, Level};
use tracing_subscriber::fmt;
use yek::{config::YekConfig, serialize_repo};

fn main() -> Result<()> {
    // 1) Parse CLI + config files:
    let mut full_config = YekConfig::init_config();

    // Validate that XML and JSON are not both enabled
    if full_config.xml && full_config.json {
        eprintln!("Error: Cannot use both --xml and --json flags together");
        std::process::exit(1);
    }

    let env_filter = if full_config.debug {
        "yek=debug,ignore=off"
    } else {
        "yek=info,ignore=off"
    };

    // 2) Initialize tracing:
    fmt::Subscriber::builder()
        .with_max_level(if full_config.debug {
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
        .with_env_filter(env_filter)
        .compact()
        .init();

    if full_config.debug {
        let config_str = serde_json::to_string_pretty(&full_config)?;
        debug!("Configuration:\n{}", config_str);
    }

    // If streaming => skip checksum + read. Just do single-thread call to serialize_repo.
    // If not streaming => run checksum + repo serialization in parallel.
    if full_config.stream {
        let (output, files) = serialize_repo(&full_config)?;
        // We print actual text to stdout:
        println!("{}", output);

        if full_config.debug {
            debug!("{} files processed (streaming).", files.len());
            debug!("Output lines: {}", output.lines().count());
        }
    } else {
        // Not streaming => run repo serialization & checksum in parallel
        let (serialization_res, checksum_res) = join(
            || serialize_repo(&full_config),
            || YekConfig::get_checksum(&full_config.input_paths),
        );

        // Handle both results
        let (output_string, files) = serialization_res?;
        let checksum = checksum_res;

        // Now set the final output file with the computed checksum
        let extension = if full_config.json {
            "json"
        } else if full_config.xml {
            "xml"
        } else {
            "txt"
        };
        let output_dir = full_config.output_dir.as_ref().ok_or_else(|| {
            anyhow::anyhow!("Output directory is required when not in streaming mode. This may indicate a configuration validation error.")
        })?;

        let final_path = Path::new(output_dir)
            .join(format!("yek-output-{}.{}", checksum, extension))
            .to_string_lossy()
            .to_string();
        full_config.output_file_full_path = Some(final_path.clone());

        // If debug, show stats
        if full_config.debug {
            let size = ByteSize::b(output_string.len() as u64);
            debug!("{} files processed", files.len());
            debug!("{} generated", size);
            debug!("{} lines generated", output_string.lines().count());
        }

        // Actually write the final output file.
        // We'll do it right here (instead of inside `serialize_repo`) to ensure we use our new final_path:
        std::fs::write(&final_path, output_string.as_bytes())?;

        // Print path to stdout (like original code did)
        println!("{}", final_path);
    }

    Ok(())
}
