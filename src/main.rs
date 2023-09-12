mod commands;
mod audio;
mod auth;
use commands::SubCommand::*;
#[tokio::main]
async fn main() {
    let cfg = commands::get_config();

    match &cfg.cmd {
        Server(command) => command.run(&cfg).await,
        Client(command) => command.run(&cfg).await,
    }
}