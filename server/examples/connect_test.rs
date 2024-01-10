use std::{ fs::File, io::Write, path::Path, process::exit };
use tracing_appender::non_blocking::{ NonBlocking, WorkerGuard };
use tracing_subscriber::fmt::SubscriberBuilder;

#[tokio::main]
async fn main() {
    let subscriber: SubscriberBuilder = tracing_subscriber::fmt();
    let non_blocking: NonBlocking;
    let _guard: WorkerGuard;
    let (non_blocking, _guard) = tracing_appender::non_blocking(std::io::stdout());

    subscriber
        .with_writer(non_blocking)
        .with_max_level(tracing::Level::DEBUG)
        .with_level(true)
        .with_line_number(true)
        .with_file(true)
        .compact()
        .init();

    tracing::info!("Logger established!");

    let mut pem = std::fs
        ::read("C:\\Users\\charlesportwoodii\\Projects\\Alaydriem\\Alaydriem.pem")
        .unwrap();

    let ca = std::fs
        ::read(
            "C:\\Users\\charlesportwoodii\\Projects\\Alaydriem\\bedrock-voice-chat\\certificates\\ca.crt"
        )
        .unwrap();

    let identity = reqwest::Identity::from_pem(&pem).unwrap();
    let certificate = reqwest::Certificate::from_pem(&ca).unwrap();

    let client = reqwest::ClientBuilder
        ::new()
        .add_root_certificate(certificate)
        .identity(identity)
        .tls_built_in_root_certs(false)
        .use_rustls_tls()
        .tls_info(true)
        .danger_accept_invalid_certs(true)
        .danger_accept_invalid_hostnames(true)
        .connection_verbose(true)
        .build()
        .unwrap();

    match client.get("https://127.0.0.1:3000/api/ping").send().await {
        Ok(result) => {
            println!("{:?}", result);
        }
        Err(e) => println!("{:?}", e),
    }
}
