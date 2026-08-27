use bvc_relay_service::config::RelayConfig;
use bvc_relay_service::logging::Logging;
use bvc_relay_service::runtime::ServiceRuntime;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "config.hcl".to_string());
    // A config that fails to parse is reported by this function returning the error,
    // which is the only reporting available before the log block has been read.
    let config = RelayConfig::from_path(&path)?;

    Logging::install(config.logger.clone());

    ServiceRuntime::new(config).start().await
}
