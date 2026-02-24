use anyhow::anyhow;
use bvc_server_lib::ApplicationConfig;
use clap::Parser;
use serde_json::Value;
use std::fs;

use super::SubCommand;

#[derive(Debug, Parser, Clone)]
#[clap(author, version, about, long_about = None)]
pub struct Config {
    // Path to bvc configuration file
    #[clap(
        global = true,
        short,
        long,
        value_parser,
        required = false,
        default_value = "config.hcl"
    )]
    pub config_file: String,

    #[clap(skip)]
    pub config: ApplicationConfig,

    /// Command to execute
    #[clap(subcommand)]
    pub cmd: SubCommand,
}

impl Config {
    // Parsing command for clap to correctly build the configuration.
    pub(super) fn get_config() -> std::sync::Arc<Self> {
        let mut data = Self::parse();

        match data.get_config_file() {
            Ok(hcl) => {
                data.config = hcl;
            }
            Err(error) => {
                println!("{}", error);
                std::process::exit(1);
            }
        };

        return std::sync::Arc::new(data);
    }

    /// Reads in the HCL configuration file
    pub fn get_config_file<'a>(&'a self) -> std::result::Result<ApplicationConfig, anyhow::Error> {
        if let Ok(config) = fs::read_to_string(&self.config_file) {
            if let Ok(hcl) = hcl::from_str::<Value>(&config.as_str()) {
                let app_config: Result<ApplicationConfig, serde_json::Error> =
                    serde_json::from_value(hcl);
                if app_config.is_ok() {
                    let acr = app_config.unwrap();
                    return Ok::<ApplicationConfig, anyhow::Error>(acr);
                } else {
                    return Err(anyhow!(app_config.unwrap_err()));
                }
            }
        }

        return Err(anyhow!("Unable to read or parse configuration file."));
    }
}
