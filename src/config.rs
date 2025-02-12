#[derive(Parser, ClapConfigFile, Clone)]
#[config_file_name = "yek"]
#[config_file_formats = "toml,yaml,json"]
pub struct YekConfig {
    /// Input directories to process
    #[config_arg(positional)]
    pub input_dirs: Vec<String>,

    /// Print version of yek
    #[config_arg(long = "version", short = 'V', action = clap::ArgAction::SetTrue)]
    pub version: bool,
    // … remaining fields unchanged