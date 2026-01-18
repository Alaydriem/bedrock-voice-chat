use bvc_server_lib::{init_crypto_provider, init_windows_timer};

mod commands;

#[tokio::main]
async fn main() {
    // Initialize Windows timer for high-precision audio
    init_windows_timer();

    // Initialize crypto provider
    if let Err(e) = init_crypto_provider() {
        eprintln!("Warning: {}", e);
    }

    // Launch via clap command chain
    commands::launch().await;
}
