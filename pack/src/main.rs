mod commands;

use commands::SubCommand::*;
#[tokio::main]
async fn main() {
    // Parse arguments with clap => config::Config struct
    let cfg = commands::get_config();

    match &cfg.cmd {
        Dev(command) => command.run(&cfg).await,
        Package(command) => command.run(&cfg).await,
    }
}
