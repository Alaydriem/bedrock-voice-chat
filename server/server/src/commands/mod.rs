mod cli_config;
pub(crate) mod server;
mod permission;
mod sub_command;
mod user;

pub use cli_config::Config;
pub use sub_command::SubCommand;

pub async fn launch() {
    let cfg = Config::get_config();

    match &cfg.cmd {
        SubCommand::Server(command) => command.run(&cfg).await,
        SubCommand::User(command) => command.run(&cfg).await,
        SubCommand::Permission(command) => command.run(&cfg).await,
    }
}
