mod commands;
mod identity;

#[tokio::main]
async fn main() {
    commands::Cli::run().await;
}
