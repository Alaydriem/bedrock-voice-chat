use clap::{Parser, Subcommand};

/// Distributed load-test orchestrator for BVC voice servers.
#[derive(Debug, Parser)]
#[clap(name = "swarm", about = "Drive real headless BVC clients across machines against one server")]
pub struct SwarmArgs {
    #[clap(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Provision bot players + login codes via the admin API, writing them to a file.
    Mint(MintArgs),
    /// Run N bots on THIS machine. Reads a JSON job from stdin, prints an AgentReport to stdout.
    /// Normally invoked by the controller over ssh; runnable by hand by piping a job JSON.
    Agent(AgentArgs),
    /// Orchestrate a full run from a config file: mint (or reuse codes), fan agents out over
    /// ssh, scrape server metrics before/after, and print an aggregate report.
    Controller(ControllerArgs),
}

#[derive(Debug, Parser)]
pub struct MintArgs {
    /// Path to the swarm config file (TOML).
    #[clap(long, default_value = "swarm.toml")]
    pub config: String,
    /// Where to write the `gamertag<TAB>code` lines.
    #[clap(long, default_value = "codes.txt")]
    pub out: String,
}

#[derive(Debug, Parser)]
pub struct AgentArgs {
    /// Path to the `bvc_client_e2e` binary on this machine.
    #[clap(long)]
    pub bin: String,
}

#[derive(Debug, Parser)]
pub struct ControllerArgs {
    /// Path to the swarm config file (TOML).
    #[clap(long, default_value = "swarm.toml")]
    pub config: String,
    /// Reuse a `gamertag<TAB>code` file instead of minting fresh codes.
    #[clap(long)]
    pub codes: Option<String>,
}
