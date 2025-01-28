use anyhow::Result;
use tracing::{debug, Level};
use tracing_subscriber::FmtSubscriber;
use yek::{config::YekConfig, serialize_repo};

fn main() -> Result<()> {
    // get the configuration from the config file and CLI args
    let full_config = YekConfig::init_config();

    // Initialize tracing based on debug flag
    FmtSubscriber::builder()
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
        .compact()
        .init();

    if full_config.debug {
        let config_str = serde_json::to_string_pretty(&full_config)?;
        debug!("Configuration:\n{}", config_str);
    }

    serialize_repo(&full_config)?;

    println!("{}", full_config.output_file_full_path);

    Ok(())
}
