#[derive(clap::Subcommand, Debug, Clone)]
pub enum SubCommand {
    Peerlink(super::peerlink::Config),
    Worlds(super::worlds::Config),
}
