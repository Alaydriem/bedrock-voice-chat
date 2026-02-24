use clap::Parser;

use super::SubCommand;
use super::super::Config as StateConfig;

#[derive(Debug, Parser, Clone)]
#[clap(author, version, about, long_about = None)]
pub struct Config {
    #[clap(subcommand)]
    pub cmd: SubCommand,
}

impl Config {
    pub async fn run<'a>(&'a self, cfg: &StateConfig) {
        match &self.cmd {
            SubCommand::Deny(command) => command.run(cfg).await,
            SubCommand::Allow(command) => command.run(cfg).await,
            SubCommand::Clear(command) => command.run(cfg).await,
            SubCommand::List(command) => command.run(cfg).await,
        }
    }
}
