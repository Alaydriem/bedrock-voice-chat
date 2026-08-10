use crate::stream::quic::connection_registry::RoutedPacket;
use crate::stream::session::{SendOutcome, SessionLink};
use anyhow::Error;
use common::traits::StreamTrait;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::mpsc;

pub(crate) struct OutputStream {
    link: Option<SessionLink>,
    packet_rx: Option<mpsc::Receiver<RoutedPacket>>,
    is_stopped: Arc<AtomicBool>,
    pub(crate) player_id: Arc<std::sync::OnceLock<String>>,
}

impl OutputStream {
    pub fn new(link: Option<SessionLink>) -> Self {
        Self {
            link,
            packet_rx: None,
            is_stopped: Arc::new(AtomicBool::new(true)),
            player_id: Arc::new(std::sync::OnceLock::new()),
        }
    }

    pub fn set_packet_receiver(&mut self, packet_rx: mpsc::Receiver<RoutedPacket>) {
        self.packet_rx = Some(packet_rx);
    }

    pub fn get_player_id(&self) -> Option<String> {
        self.player_id.get().cloned()
    }

}

impl StreamTrait for OutputStream {
    fn is_stopped(&self) -> bool {
        self.is_stopped.load(Ordering::Relaxed)
    }

    async fn stop(&mut self) -> Result<(), Error> {
        tracing::info!("Stopping QUIC output stream");
        self.is_stopped.store(true, Ordering::Relaxed);
        Ok(())
    }

    async fn start(&mut self) -> Result<(), Error> {
        tracing::info!("Starting session output stream");
        self.is_stopped.store(false, Ordering::Relaxed);

        if let (Some(link), Some(mut packet_rx)) = (self.link.clone(), self.packet_rx.take()) {
            while let Some(routed) = packet_rx.recv().await {
                let payload = match routed {
                    RoutedPacket::Serialized(bytes) => bytes,
                };

                let player = self.get_player_id().unwrap_or_else(|| "unknown".into());

                match link.send(payload) {
                    SendOutcome::Ok => {}
                    SendOutcome::ConnectionClosed(emsg) => {
                        tracing::error!("datagram_send_closed player={} err={}", player, emsg);
                        break;
                    }
                    SendOutcome::Capacity(emsg) => {
                        tracing::debug!(
                            "datagram send capacity issue player={} err={}",
                            player,
                            emsg
                        );
                    }
                    SendOutcome::Other(emsg) => {
                        tracing::debug!("datagram send error player={} err={}", player, emsg);
                    }
                    SendOutcome::Fatal(emsg) => {
                        tracing::error!(
                            "datagram_send_query_failed player={} err={}",
                            player,
                            emsg
                        );
                        break;
                    }
                }
            }
        }

        self.is_stopped.store(true, Ordering::Relaxed);
        Ok(())
    }

    async fn metadata(&mut self, key: String, value: String) -> Result<(), Error> {
        tracing::info!(
            "Setting metadata for QUIC output stream: {} = {}",
            key,
            value
        );
        Ok(())
    }
}
