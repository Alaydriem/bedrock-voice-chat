
extern crate common;
use tokio;

mod config;
mod commands;
mod rs;
use commands::SubCommand::*;

mod built_info {
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

#[macro_use]
extern crate rocket;

#[tokio::main]
async fn main() {
    // Parse arguments with clap => config::Config struct
    let cfg = commands::State::get_config();

    match &cfg.cmd {
        Server(command) => command.run(&cfg).await,
    }
}