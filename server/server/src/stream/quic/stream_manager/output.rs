use common::curia;
use crate::stream::quic::connection::RoutedPacket;
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
    // Microseconds the send loop waits after the first queued datagram before flushing,
    // so concurrent speakers' frames share one transport flush. 0 disables the wait.
    send_batch_wait_micros: u64,
    pub(crate) player_id: Arc<std::sync::OnceLock<String>>,
}

impl OutputStream {
    pub fn new(link: Option<SessionLink>, send_batch_wait_micros: u64) -> Self {
        Self {
            link,
            packet_rx: None,
            is_stopped: Arc::new(AtomicBool::new(true)),
            send_batch_wait_micros,
            player_id: Arc::new(std::sync::OnceLock::new()),
        }
    }

    pub fn set_packet_receiver(&mut self, packet_rx: mpsc::Receiver<RoutedPacket>) {
        self.packet_rx = Some(packet_rx);
    }

    // Borrows rather than clones. The send loop below names the player only on its error arms, and
    // it runs once per outbound datagram, so resolving this by clone allocated a `String` per
    // datagram to build a log line that almost never happens.
    fn log_label(&self) -> &str {
        self.player_id.get().map(String::as_str).unwrap_or("unknown")
    }
}

impl StreamTrait for OutputStream {
    fn is_stopped(&self) -> bool {
        self.is_stopped.load(Ordering::Relaxed)
    }

    async fn stop(&mut self) -> Result<(), Error> {
        curia::info!("Stopping QUIC output stream");
        self.is_stopped.store(true, Ordering::Relaxed);
        Ok(())
    }

    async fn start(&mut self) -> Result<(), Error> {
        curia::info!("Starting session output stream");
        self.is_stopped.store(false, Ordering::Relaxed);

        if let (Some(link), Some(mut packet_rx)) = (self.link.clone(), self.packet_rx.take()) {
            let batcher =
                crate::stream::quic::stream_manager::SendBatcher::new(self.send_batch_wait_micros);
            let mut batch: Vec<bytes::Bytes> = Vec::with_capacity(32);
            while batcher.collect(&mut packet_rx, &mut batch).await.is_some() {
                match link.send_batch(&mut batch) {
                    SendOutcome::Ok => {}
                    SendOutcome::ConnectionClosed(emsg) => {
                        curia::error!(
                            "datagram_send_closed player={} err={}",
                            self.log_label(),
                            emsg
                        );
                        break;
                    }
                    SendOutcome::Capacity(emsg) => {
                        curia::debug!(
                            "datagram send capacity issue player={} err={}",
                            self.log_label(),
                            emsg
                        );
                    }
                    SendOutcome::Other(emsg) => {
                        curia::debug!(
                            "datagram send error player={} err={}",
                            self.log_label(),
                            emsg
                        );
                    }
                    SendOutcome::Fatal(emsg) => {
                        curia::error!(
                            "datagram_send_query_failed player={} err={}",
                            self.log_label(),
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
        curia::info!(
            "Setting metadata for QUIC output stream: {} = {}",
            key,
            value
        );
        Ok(())
    }
}
