use anyhow::Result;
use bytesize::ByteSize;
use tracing::{debug, Level};
use tracing_subscriber::fmt;
use yek::{config::YekConfig, serialize_repo};

fn main() -> Result<()> {
    // get the configuration from the config file and CLI args
    let full_config = YekConfig::init_config();

    // Initialize tracing based on debug flag
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
        .with_env_filter("yek=debug,ignore=off")
        .compact()
        .init();

    if full_config.debug {
        let config_str = serde_json::to_string_pretty(&full_config)?;
        debug!("Configuration:\n{}", config_str);
    }

    let (output, files) = serialize_repo(&full_config)?;

    // if stream is true, print the actual output
    if full_config.stream {
        println!("{}", output);

    // if it is a terminal, print the output file path
    } else {
        // if debug mode print output size and number of lines and number of files
        if full_config.debug {
            let size = ByteSize::b(output.len() as u64);
            debug!("{} files processed", files.len());
            debug!("{} generated", size); // human readable size
            debug!("{} lines generated", output.lines().count());
        }

        println!("{}", full_config.output_file_full_path);
    }

    Ok(())
}
