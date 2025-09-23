use anyhow::Result;
use yek::{config::YekConfig, main_new::main_new};

fn main() -> Result<()> {
    // Use the new architecture
    main_new()
}
