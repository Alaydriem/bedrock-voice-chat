mod banish;
use super::Config as StateConfig;
use clap::Parser;

#[derive(clap::Subcommand, Debug, Clone)]
pub enum SubCommand {
    /// Banishes a user
    Banish(banish::Config),
}

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
        }
    }
}
