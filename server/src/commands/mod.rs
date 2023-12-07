use std::sync::Arc;
use clap::Parser;

mod server;

#[derive(clap::Subcommand, Debug, Clone)]
pub enum SubCommand {
    /// Start the BVC Server
    Server(server::Config),
}

#[derive(Debug, Parser, Clone)]
#[clap(author, version, about, long_about = None)]
pub struct State {
    /// Command to execute
    #[clap(subcommand)]
    pub cmd: SubCommand,
}

impl State {
    // Parsing command for clap to correctly build the configuration.
    pub fn get_config() -> Arc<State> {
        return Arc::new(State::parse());
    }
}
