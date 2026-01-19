use super::Config as StateConfig;
use bvc_server_lib::ServerRuntime;

use clap::Parser;
use std::process::exit;

/// Starts the BVC Server
#[derive(Debug, Parser, Clone)]
#[clap(author, version, about, long_about = None)]
pub struct Config {}

impl Config {
    /// Starts the BVC Server using ServerRuntime
    pub async fn run(&self, cfg: &StateConfig) {
        let mut runtime = match ServerRuntime::new(cfg.config.clone()) {
            Ok(rt) => rt,
            Err(e) => {
                eprintln!("Failed to create server runtime: {}", e);
                exit(1);
            }
        };

        // Set up CTRL+C handler to request graceful shutdown
        let shutdown_flag = runtime.shutdown_flag();
        tokio::spawn(async move {
            if let Ok(()) = tokio::signal::ctrl_c().await {
                eprintln!("\nReceived CTRL+C, shutting down...");
                shutdown_flag.store(true, std::sync::atomic::Ordering::SeqCst);
            }
        });

        if let Err(e) = runtime.start_async().await {
            eprintln!("Server error: {}", e);
            exit(1);
        }
    }
}
