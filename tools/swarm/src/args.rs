use clap::{Parser, Subcommand};

use crate::scene::Scenario;

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
    /// Stage a named roster against a server and hold it there, for screenshots and for
    /// looking at a screen that is hard to populate by hand.
    Scene(SceneCommandArgs),
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
pub struct SceneCommandArgs {
    /// Path to the scene config file (TOML).
    #[clap(long, default_value = "scene.toml")]
    pub config: String,
    /// Which picture to compose.
    #[clap(long, value_enum)]
    pub scenario: Scenario,
    /// Staged players to connect for real so they transmit. Repeat the flag or pass a
    /// comma-separated list. Each costs one process, and all of them talk at once.
    #[clap(long, value_delimiter = ',')]
    pub speaking: Vec<String>,
    /// Also feed the world's chat: players talking to each other, and system lines like
    /// deaths and advancements. Needs no admin identity and no processes.
    #[clap(long)]
    pub chat: bool,
    /// Print this many chat lines and exit without contacting the server. For settling on the
    /// conversation before a scene is staged.
    #[clap(long)]
    pub chat_preview: Option<usize>,
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
