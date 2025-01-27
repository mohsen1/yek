use anyhow::Result;
use std::path::Path;
use yek::{config::YekConfig, serialize_repo};

fn main() -> Result<()> {
    // get list of directories from process args
    // any arg without -- is a directory
    let args: Vec<String> = std::env::args().collect();
    let dirs: Vec<String> = args
        .iter()
        .skip(1)
        .filter(|arg| !arg.starts_with("--"))
        .map(|s| s.to_string())
        .collect();

    // get the configuration from the config file and CLI args
    let full_config = YekConfig::default();

    for dir in &dirs {
        let path = Path::new(dir);
        serialize_repo(path, &full_config)?;
    }

    Ok(())
}
