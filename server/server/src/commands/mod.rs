use anyhow::anyhow;
use lib_bvc_server::ApplicationConfig;
use clap::Parser;
use serde_json::Value;
use std::fs;
use std::{process::exit, sync::Arc};

pub(crate) mod server;
mod user;
#[derive(clap::Subcommand, Debug, Clone)]
pub enum SubCommand {
    /// Start the BVC Server
    Server(server::Config),
    User(user::Config),
}

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

pub async fn launch() {
    // Parse arguments with clap => config::Config struct
    let cfg = Config::get_config();

    match &cfg.cmd {
        SubCommand::Server(command) => command.run(&cfg).await,
        SubCommand::User(command) => command.run(&cfg).await,
    }
}

impl Config {
    // Parsing command for clap to correctly build the configuration.
    fn get_config() -> Arc<Self> {
        let mut data = Self::parse();

        match data.get_config_file() {
            Ok(hcl) => {
                data.config = hcl;
            }
            Err(error) => {
                println!("{}", error);
                exit(1);
            }
        };

        return Arc::new(data);
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
