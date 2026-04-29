use anyhow::anyhow;
use bvc_server_lib::ApplicationConfig;
use clap::Parser;
use serde_json::Value;
use std::fs;
use std::{process::exit, sync::Arc};

pub(crate) mod admin;
pub(crate) mod admin_api_client;
pub(crate) mod identity;
pub(crate) mod login;
pub(crate) mod logout;
mod permission;
pub(crate) mod server;
mod user;
pub(crate) mod whoami;

#[derive(clap::Subcommand, Debug, Clone)]
pub enum SubCommand {
    /// Start the BVC Server
    Server(server::Config),
    /// Authenticate the CLI against a BVC server
    Login(login::Config),
    /// Remove a stored CLI identity
    Logout(logout::Config),
    /// Print the active identity
    Whoami(whoami::Config),
    /// Administrative bootstrapping (DB-direct, server-host only)
    Admin(admin::Config),
    User(user::Config),
    Permission(permission::Config),
}

#[derive(Debug, Parser, Clone)]
#[clap(author, version, about, long_about = None)]
pub struct Cli {
    /// Path to bvc configuration file (only required for `server` and `admin bootstrap`)
    #[clap(
        global = true,
        short,
        long,
        value_parser,
        required = false,
        default_value = "config.hcl"
    )]
    pub config_file: String,

    /// BVC server URL (used by `login`; admin commands read from the stored identity)
    #[clap(global = true, long, env = "BVC_SERVER_URL", default_value = "https://127.0.0.1:3000")]
    pub server_url: String,

    /// Active identity selector, format `<gamertag>:<game>`. Falls back to BVC_IDENTITY env or sole-stored-identity
    #[clap(global = true, long, env = "BVC_IDENTITY")]
    pub identity: Option<String>,

    #[clap(skip)]
    pub config: ApplicationConfig,

    /// Command to execute
    #[clap(subcommand)]
    pub cmd: SubCommand,
}

impl Cli {
    pub async fn run() {
        let cfg = Self::get_config();

        match &cfg.cmd {
            SubCommand::Server(command) => command.run(&cfg).await,
            SubCommand::Admin(command) => command.run(&cfg).await,
            SubCommand::Login(command) => command.run(&cfg.server_url).await,
            SubCommand::Logout(command) => command.run(cfg.identity.as_deref()).await,
            SubCommand::Whoami(command) => command.run(cfg.identity.as_deref()).await,
            SubCommand::User(command) => command.run(&cfg).await,
            SubCommand::Permission(command) => command.run(&cfg).await,
        }
    }

    /// Returns whether the current subcommand needs the HCL config file loaded.
    fn requires_config_file(&self) -> bool {
        matches!(self.cmd, SubCommand::Server(_) | SubCommand::Admin(_))
    }

    fn get_config() -> Arc<Self> {
        let mut data = Self::parse();

        if data.requires_config_file() {
            match data.get_config_file() {
                Ok(hcl) => {
                    data.config = hcl;
                }
                Err(error) => {
                    println!("{}", error);
                    exit(1);
                }
            };
        }

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
