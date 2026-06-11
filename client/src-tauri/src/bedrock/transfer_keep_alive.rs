use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use common::request::bedrock::TransferTargetRequest;
use common::traits::StreamTrait;
use log::{error, info};
use tokio::sync::oneshot;
use tokio::task::{AbortHandle, JoinHandle};

pub struct TransferKeepAlive {
    server_url: String,
    xuid: String,
    host: String,
    port: u16,
    client: reqwest::Client,
    jobs: Vec<AbortHandle>,
    shutdown: Arc<AtomicBool>,
    shutdown_tx: Option<oneshot::Sender<()>>,
}

impl TransferKeepAlive {
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
            jobs: vec![],
            shutdown: Arc::new(AtomicBool::new(false)),
            shutdown_tx: None,
        }
    }
}

impl StreamTrait for TransferKeepAlive {
    async fn start(&mut self) -> Result<(), anyhow::Error> {
        if !self.jobs.is_empty() {
            return Err(anyhow::anyhow!("Keepalive already running"));
        }
        let _ = self.shutdown.store(false, Ordering::Relaxed);

        let mut jobs = vec![];

        match self.listener(self.shutdown.clone()) {
            Ok(job) => jobs.push(job),
            Err(e) => {
                error!("Keepalive listener encountered an error: {:?}", e);
                return Err(e);
            }
        };

        self.jobs = jobs.iter().map(|h| h.abort_handle()).collect();
        Ok(())
    }

    async fn stop(&mut self) -> Result<(), anyhow::Error> {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
        let _ = self.shutdown.store(true, Ordering::Relaxed);

        let _ = tokio::time::sleep(Duration::from_millis(100)).await;

        for job in &self.jobs {
            job.abort();
        }

        self.jobs = vec![];
        info!("Transfer keepalive stopped");
        Ok(())
    }

    fn is_stopped(&self) -> bool {
        self.jobs.len() == 0
    }

    async fn metadata(&mut self, _key: String, _value: String) -> Result<(), anyhow::Error> {
        Ok(())
    }
}

impl TransferKeepAlive {
    fn listener(&mut self, shutdown: Arc<AtomicBool>) -> Result<JoinHandle<()>, anyhow::Error> {
        let (shutdown_tx, mut shutdown_rx) = oneshot::channel::<()>();
        self.shutdown_tx = Some(shutdown_tx);

        let server_url = self.server_url.clone();
        let xuid = self.xuid.clone();
        let host = self.host.clone();
        let port = self.port;
        let client = self.client.clone();

        let handle = tokio::spawn(async move {
            loop {
                if shutdown.load(Ordering::Relaxed) {
                    break;
                }

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
                    _ = &mut shutdown_rx => {
                        info!("Keepalive shutdown signal received");
                        break;
                    }
                }
            }
        });

        Ok(handle)
    }
}
