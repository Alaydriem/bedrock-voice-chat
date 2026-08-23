mod api_client;
mod api_error;
pub mod bootstrap;
pub mod generate_code;

pub use api_client::AdminApiClient;
pub use api_error::AdminApiError;

use clap::Parser;

use crate::commands::Cli;

#[derive(clap::Subcommand, Debug, Clone)]
pub enum SubCommand {
    /// Grant the `admin` permission to the very first operator. DB-direct, runs only on the server host.
    Bootstrap(bootstrap::Config),
    /// Generate a one-time login code for a player. DB-direct; creates the player if missing.
    GenerateCode(generate_code::Config),
}

#[derive(Debug, Parser, Clone)]
#[clap(author, version, about = "Administrative bootstrapping", long_about = None)]
pub struct Config {
    #[clap(subcommand)]
    pub cmd: SubCommand,
}

impl Config {
    pub async fn run<'a>(&'a self, cfg: &Cli) {
        match &self.cmd {
            SubCommand::Bootstrap(command) => command.run(cfg).await,
            SubCommand::GenerateCode(command) => command.run(cfg).await,
        }
    }
}
