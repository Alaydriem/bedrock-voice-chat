use super::Cli;
use bvc_server_lib::BvcServer;

use clap::Parser;
use std::process::exit;

#[derive(Debug, Parser, Clone)]
#[clap(author, version, about, long_about = None)]
pub struct Config {}

impl Config {
    pub async fn run(&self, cfg: &Cli) {
        let mut runtime = match BvcServer::new(cfg.config.clone()) {
            Ok(rt) => rt,
            Err(e) => {
                eprintln!("Failed to create server runtime: {}", e);
                exit(1);
            }
        };

        if let Err(e) = runtime.start().await {
            eprintln!("Server error: {}", e);
            exit(1);
        }
    }
}
