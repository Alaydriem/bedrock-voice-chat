use anyhow::anyhow;
use bvc_server_lib::ApplicationConfig;
use clap::Parser;
use std::fs;
use std::{process::exit, sync::Arc};

pub(crate) mod admin;
pub(crate) mod admin_api_client;
pub(crate) mod admin_api_error;
pub(crate) mod login;
pub(crate) mod logout;
mod permission;
mod relay;
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
    /// Cross-server peering diagnostics
    Relay(relay::Config),
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
    #[clap(
        global = true,
        long,
        env = "BVC_SERVER",
        default_value = "https://127.0.0.1:3000"
    )]
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
            SubCommand::Relay(command) => command.run(&cfg).await,
        }
    }

    /// Returns whether the current subcommand needs the HCL config file loaded.
    fn requires_config_file(&self) -> bool {
        matches!(self.cmd, SubCommand::Server(_) | SubCommand::Admin(_))
    }

    fn get_config() -> Arc<Self> {
        let mut data = Self::parse();

        if data.requires_config_file() {
            let base = if std::path::Path::new(&data.config_file).exists() {
                match data.get_config_file() {
                    Ok(hcl) => hcl,
                    Err(error) => {
                        println!("{}", error);
                        exit(1);
                    }
                }
            } else {
                // No config file: defaults + environment. Anything genuinely
                // required and absent (e.g. TLS cert paths) fails downstream
                // with its own specific error.
                println!(
                    "Configuration file {} not found; using defaults + environment variables",
                    &data.config_file
                );
                ApplicationConfig::default()
            };
            data.config = Self::apply_env_overrides(base);
        }

        return Arc::new(data);
    }

    /// Applies the curated BVC_* environment overrides on top of the parsed
    /// (or defaulted) configuration. A malformed value is fatal.
    fn apply_env_overrides(config: ApplicationConfig) -> ApplicationConfig {
        match bvc_server_lib::config::EnvOverrides::from_env().apply(config) {
            Ok(config) => config,
            Err(error) => {
                println!("{}", error);
                exit(1);
            }
        }
    }

    /// Reads in the HCL configuration file
    pub fn get_config_file<'a>(&'a self) -> std::result::Result<ApplicationConfig, anyhow::Error> {
        let content = fs::read_to_string(&self.config_file).map_err(|e| {
            anyhow!(
                "Unable to read configuration file {}: {e}",
                &self.config_file
            )
        })?;
        ApplicationConfig::from_hcl_str(&content)
    }
}
