use super::server;
use super::user;
use super::permission;

#[derive(clap::Subcommand, Debug, Clone)]
pub enum SubCommand {
    /// Start the BVC Server
    Server(server::Config),
    User(user::Config),
    /// Manage player permissions
    Permission(permission::Config),
}
