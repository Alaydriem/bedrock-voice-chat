#[derive(clap::Subcommand, Debug, Clone)]
pub enum SubCommand {
    Peerlink(super::peerlink::Config),
    Worlds(super::worlds::Config),
    Pair(super::pair::Config),
    Peers(super::peers::Config),
    Unpair(super::unpair::Config),
}
