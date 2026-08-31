use super::SubCommand;
use crate::commands::Cli;
use clap::Parser;

#[derive(Debug, Parser, Clone)]
#[clap(author, version, about, long_about = None)]
pub struct Config {
    #[clap(subcommand)]
    pub cmd: SubCommand,
}

impl Config {
    pub async fn run<'a>(&'a self, cfg: &Cli) {
        match &self.cmd {
            SubCommand::Peerlink(command) => command.run(cfg).await,
            SubCommand::Worlds(command) => command.run(cfg).await,
            SubCommand::Pair(command) => command.run(cfg).await,
            SubCommand::Peers(command) => command.run(cfg).await,
            SubCommand::Unpair(command) => command.run(cfg).await,
        }
    }
}
