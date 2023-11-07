use clap::Parser;
use std::sync::Arc;
pub(crate) mod dev;
pub(crate) mod package;

#[derive(clap::Subcommand, Debug, Clone)]
pub enum SubCommand {
    /// Creates a bedrock development environment by copying the package to com.mojang // development_
    Dev(dev::Config),
    /// Packages the packs into .mcpack
    Package(package::Config),
}

#[derive(Debug, Parser, Clone)]
#[clap(author, version, about, long_about = None)]
pub struct Command {
    /// Command to execute
    #[clap(subcommand)]
    pub cmd: SubCommand,
}

pub fn get_config() -> Arc<Command> {
    return Arc::new(Command::parse());
}
