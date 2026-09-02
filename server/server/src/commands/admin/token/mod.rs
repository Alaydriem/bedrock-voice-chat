mod list;
mod mint;
mod revoke;
mod rotate;
mod show;
mod sub_command;

pub use sub_command::SubCommand;

use clap::Parser;

use crate::commands::Cli;

#[derive(Debug, Parser, Clone)]
#[clap(author, version, about = "Manage game server access tokens", long_about = None)]
pub struct Config {
    #[clap(subcommand)]
    pub cmd: SubCommand,
}

impl Config {
    pub async fn run<'a>(&'a self, cfg: &Cli) {
        match &self.cmd {
            SubCommand::List(command) => command.run(cfg).await,
            SubCommand::Mint(command) => command.run(cfg).await,
            SubCommand::Revoke(command) => command.run(cfg).await,
            SubCommand::Rotate(command) => command.run(cfg).await,
            SubCommand::Show(command) => command.run(cfg).await,
        }
    }
}
