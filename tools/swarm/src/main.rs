mod agent;
mod args;
mod bot_proc;
mod bridge_codec;
mod config;
mod controller;
mod job;
mod lxd_client;
mod lxd_config;
mod metrics_scrape;
mod minter;
mod position_sender;
mod report;
mod target_spec;
mod tone;

use std::io::Read;

use clap::Parser;

use crate::agent::SwarmAgent;
use crate::args::{Command, MintArgs, SwarmArgs};
use crate::config::SwarmConfig;
use crate::controller::SwarmController;
use crate::job::AgentJob;
use crate::minter::CodeMinter;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    match SwarmArgs::parse().command {
        Command::Mint(a) => run_mint(a).await,
        Command::Agent(a) => {
            let mut buf = String::new();
            std::io::stdin()
                .read_to_string(&mut buf)
                .map_err(|e| anyhow::anyhow!("reading job from stdin: {}", e))?;
            let job: AgentJob = serde_json::from_str(&buf)
                .map_err(|e| anyhow::anyhow!("parsing agent job: {}", e))?;
            let report = SwarmAgent::new(a.bin, job).run().await?;
            println!("{}", serde_json::to_string(&report)?);
            Ok(())
        }
        Command::Controller(a) => {
            let config = SwarmConfig::load(&a.config)?;
            SwarmController::prepare(config, a.codes).await?.run().await
        }
    }
}

// Kept out of `main`'s match arm only because it needs its own error context;
// still a thin dispatcher to CodeMinter, no orchestration logic of its own.
async fn run_mint(a: MintArgs) -> Result<(), anyhow::Error> {
    let config = SwarmConfig::load(&a.config)?;
    let names: Vec<String> = (0..config.total_bots()).map(|i| config.bot_name(i)).collect();
    let minter = CodeMinter::new(&config)?;
    let codes = minter.mint(&names, config.duration_secs + 300).await?;
    let body = codes
        .iter()
        .map(|(n, c)| format!("{}\t{}", n, c))
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(&a.out, body).map_err(|e| anyhow::anyhow!("writing {}: {}", a.out, e))?;
    eprintln!("wrote {} codes to {}", codes.len(), a.out);
    Ok(())
}
