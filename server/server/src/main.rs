mod commands;

#[tokio::main]
async fn main() {
    commands::Cli::run().await;
}
