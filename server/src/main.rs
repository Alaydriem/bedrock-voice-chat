extern crate common;
use tokio;

mod commands;
mod config;
mod rs;
use commands::SubCommand::*;

mod built_info {
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

#[macro_use]
extern crate rocket;

#[tokio::main]
async fn main() {
    let app = commands::launch().await;
}
