#[derive(clap::Subcommand, Debug, Clone)]
pub enum SubCommand {
    Deny(super::deny::Config),
    Allow(super::allow::Config),
    Clear(super::clear::Config),
    List(super::list::Config),
}
