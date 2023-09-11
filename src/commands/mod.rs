use clap::Parser;
use std::sync::Arc;
pub(crate) mod server;
pub(crate) mod client;

#[derive(clap::Subcommand, Debug, Clone)]
pub enum SubCommand {
    /// Starts the Bedrock Voice Chat server
    Server(server::Config),
    /// Starts the Bedrock Voice Chat command line client
    Client(client::Config),
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
