use std::time::Duration;

use common::request::bedrock::TransferTargetRequest;
use common::traits::StreamTrait;
use tokio::sync::watch;
use tokio::task::AbortHandle;

pub struct TransferKeepalive {
    server_url: String,
    xuid: String,
    host: String,
    port: u16,
    client: reqwest::Client,
    abort_handle: Option<AbortHandle>,
    shutdown_tx: Option<watch::Sender<bool>>,
}

impl TransferKeepalive {
    pub fn new(
        server_url: String,
        xuid: String,
        host: String,
        port: u16,
        client: reqwest::Client,
    ) -> Self {
        Self {
            server_url,
            xuid,
            host,
            port,
            client,
            abort_handle: None,
            shutdown_tx: None,
        }
    }

    async fn start_keepalive_loop(
        &self,
        mut shutdown_rx: watch::Receiver<bool>,
    ) -> Result<AbortHandle, anyhow::Error> {
        let server_url = self.server_url.clone();
        let xuid = self.xuid.clone();
        let host = self.host.clone();
        let port = self.port;
        let client = self.client.clone();

        let handle = tokio::spawn(async move {
            loop {
                let request = TransferTargetRequest {
                    xuid: xuid.clone(),
                    host: host.clone(),
                    port,
                };

                let url = format!("{}/api/bedrock/transfer", server_url);
                let mut success = false;
                for attempt in 0..3 {
                    match client.post(&url).json(&request).send().await {
                        Ok(resp) if resp.status().is_success() => {
                            log::debug!("Transfer keepalive registered: {}:{}", host, port);
                            success = true;
                            break;
                        }
                        Ok(resp) => {
                            log::warn!(
                                "Transfer keepalive attempt {} failed: HTTP {}",
                                attempt + 1,
                                resp.status()
                            );
                        }
                        Err(e) => {
                            log::error!(
                                "Transfer keepalive attempt {} error: {}",
                                attempt + 1,
                                e
                            );
                        }
                    }
                    if attempt < 2 {
                        tokio::time::sleep(Duration::from_secs(2u64.pow(attempt as u32))).await;
                    }
                }
                if !success {
                    log::error!("Transfer keepalive failed after 3 attempts");
                }

                tokio::select! {
                    _ = tokio::time::sleep(Duration::from_secs(300)) => {}
                    _ = shutdown_rx.wait_for(|&v| v) => {
                        log::info!("Keepalive shutdown signal received");
                        break;
                    }
                }
            }
        });

        Ok(handle.abort_handle())
    }
}

impl StreamTrait for TransferKeepalive {
    async fn start(&mut self) -> Result<(), anyhow::Error> {
        if self.abort_handle.is_some() {
            return Err(anyhow::anyhow!("Keepalive already running"));
        }

        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        self.shutdown_tx = Some(shutdown_tx);

        let handle = self.start_keepalive_loop(shutdown_rx).await?;
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
        log::info!("Transfer keepalive stopped");
        Ok(())
    }

    fn is_stopped(&self) -> bool {
        self.abort_handle.is_none()
    }

    async fn metadata(&mut self, _key: String, _value: String) -> Result<(), anyhow::Error> {
        Ok(())
    }
}
