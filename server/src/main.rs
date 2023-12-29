extern crate common;
use tokio;

mod commands;
mod config;
mod rs;

mod built_info {
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

#[macro_use]
extern crate rocket;

#[tokio::main]
async fn main() {
    let _app = commands::launch().await;
}
