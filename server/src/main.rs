extern crate common;
use tokio;

mod commands;
mod config;
mod rs;

mod built_info {
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

#[macro_use]
extern crate rocket;

#[tokio::main]
async fn main() {
    _ = s2n_quic::provider::tls::rustls::rustls::crypto::aws_lc_rs::default_provider()
        .install_default();
    let _app = commands::launch().await;
}
