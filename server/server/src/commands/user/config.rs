use clap::Parser;

use super::SubCommand;
use super::super::Config as StateConfig;

/// Starts the BVC Server
#[derive(Debug, Parser, Clone)]
#[clap(author, version, about, long_about = None)]
pub struct Config {
    /// Command to execute
    #[clap(subcommand)]
    pub cmd: SubCommand,
}

impl Config {
    pub async fn run<'a>(&'a self, cfg: &StateConfig) {
        match &self.cmd {
            SubCommand::Banish(command) => command.run(&cfg).await,
            SubCommand::Add(command) => command.run(&cfg).await,
        }
    }
}
