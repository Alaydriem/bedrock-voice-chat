#[derive(clap::Subcommand, Debug, Clone)]
pub enum SubCommand {
    /// Banishes a user
    Banish(super::banish::Config),
    /// Adds a user
    Add(super::add::Config),
}
