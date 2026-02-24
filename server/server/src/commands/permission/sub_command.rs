#[derive(clap::Subcommand, Debug, Clone)]
pub enum SubCommand {
    /// Deny a permission for a player
    Deny(super::deny::Config),
    /// Explicitly allow a permission for a player
    Allow(super::allow::Config),
    /// Clear a permission override (fall back to config default)
    Clear(super::clear::Config),
    /// List permission overrides for a player
    List(super::list::Config),
}
