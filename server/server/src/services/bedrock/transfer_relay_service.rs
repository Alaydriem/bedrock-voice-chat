use common::bedrock_protocol::TransferPacket;
use common::bedrock_server::{BedrockServer, ServerConfig, StartGameConfig};
use common::traits::StreamTrait;
use moka::future::Cache;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;
use tokio::sync::watch;
use tokio::task::AbortHandle;

use super::InvalidAttemptEntry;
use super::TransferTargetCache;

const MAX_INVALID_ATTEMPTS: u32 = 5;

pub struct TransferRelayService {
    bind_port: u16,
    cache: TransferTargetCache,
    abort_handle: Option<AbortHandle>,
    shutdown_tx: Option<watch::Sender<bool>>,
}

impl TransferRelayService {
    pub fn new(bind_port: u16, cache: TransferTargetCache) -> Self {
        Self {
            bind_port,
            cache,
            abort_handle: None,
            shutdown_tx: None,
        }
    }

    fn build_invalid_attempt_cache() -> Cache<String, Arc<InvalidAttemptEntry>> {
        Cache::builder()
            .time_to_live(Duration::from_secs(300))
            .max_capacity(10_000)
            .build()
    }

    async fn start_server_loop(
        &self,
        mut shutdown_rx: watch::Receiver<bool>,
    ) -> Result<AbortHandle, anyhow::Error> {
        let config = ServerConfig {
            bind: format!("0.0.0.0:{}", self.bind_port).parse()?,
            motd: "BVC Transfer".to_string(),
            sub_motd: "Bedrock Voice Chat".to_string(),
            ..Default::default()
        };

        let mut server = BedrockServer::bind(config).await?;
        tracing::info!(
            "Bedrock transfer relay listening on {}",
            server.local_addr()
        );

        let cache = self.cache.clone();
        let ip_blocks = Self::build_invalid_attempt_cache();

        let handle = tokio::spawn(async move {
            loop {
                tokio::select! {
                    conn = server.accept() => {
                        let Some(mut conn) = conn else {
                            break;
                        };

                        let cache = cache.clone();
                        let ip_blocks = ip_blocks.clone();
                        tokio::spawn(async move {
                            let player_name = conn.player().name.clone();
                            let xuid = conn.player().xuid.clone();
                            tracing::info!(
                                "Transfer relay: player connected: {} (XUID: {})",
                                player_name,
                                xuid,
                            );

                            // Looked up by the name the verified Bedrock login chain
                            // proved, which is the same identity the caller's certificate
                            // CN carries. The xuid stays for logging and the
                            // invalid-attempt counter, neither of which needs to match the
                            // cache key.
                            let target = cache.get(&player_name).await;

                            match target {
                                Some(target) => {
                                    let start_game =
                                        StartGameConfig::for_version(conn.protocol_version())
                                            .into_packet();
                                    if let Err(e) = conn.send_packet(&start_game).await {
                                        tracing::error!("Failed to send StartGame to {}: {}", player_name, e);
                                        return;
                                    }

                                    let transfer = TransferPacket {
                                        server_address: target.host.clone(),
                                        server_port: target.port,
                                        reload_world: false,
                                        gatherings_configuration: None,
                                    };
                                    if let Err(e) = conn.send_packet(&transfer).await {
                                        tracing::error!("Failed to send TransferPacket to {}: {}", player_name, e);
                                        return;
                                    }
                                    tracing::info!(
                                        "Transferred {} -> {}:{}",
                                        player_name,
                                        target.host,
                                        target.port,
                                    );
                                }
                                None => {
                                    tracing::warn!(
                                        "No transfer target for {} (XUID: {}), disconnecting",
                                        player_name,
                                        xuid,
                                    );

                                    let entry = ip_blocks.get_with(
                                        xuid.clone(),
                                        async { Arc::new(InvalidAttemptEntry { count: AtomicU32::new(0) }) },
                                    ).await;
                                    let attempts = entry.count.fetch_add(1, Ordering::Relaxed) + 1;
                                    if attempts >= MAX_INVALID_ATTEMPTS {
                                        tracing::warn!(
                                            "XUID {} blocked after {} invalid attempts",
                                            xuid,
                                            attempts,
                                        );
                                    }

                                    let _ = conn.disconnect("No transfer target registered. Please start a proxy or realms session in BVC first.").await;
                                }
                            }
                        });
                    }
                    _ = shutdown_rx.wait_for(|&v| v) => {
                        tracing::info!("Transfer relay shutdown signal received");
                        break;
                    }
                }
            }
        });

        Ok(handle.abort_handle())
    }
}

impl StreamTrait for TransferRelayService {
    async fn start(&mut self) -> Result<(), anyhow::Error> {
        if self.abort_handle.is_some() {
            return Err(anyhow::anyhow!("Transfer relay already running"));
        }

        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        self.shutdown_tx = Some(shutdown_tx);

        let handle = self.start_server_loop(shutdown_rx).await?;
        self.abort_handle = Some(handle);

        Ok(())
    }

    async fn stop(&mut self) -> Result<(), anyhow::Error> {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(true);
        }

        if let Some(task) = &self.abort_handle {
            task.abort();
        }

        self.abort_handle = None;
        tracing::info!("Transfer relay service stopped");
        Ok(())
    }

    fn is_stopped(&self) -> bool {
        self.abort_handle.is_none()
    }

    async fn metadata(&mut self, _key: String, _value: String) -> Result<(), anyhow::Error> {
        Ok(())
    }
}
