use crate::stream::quic::connection_registry::RoutedPacket;
use anyhow::Error;
use bytes::Bytes;
use common::s2n_quic::Connection;
use common::traits::StreamTrait;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::mpsc;

pub(crate) struct OutputStream {
    connection: Option<Arc<Connection>>,
    packet_rx: Option<mpsc::Receiver<RoutedPacket>>,
    is_stopped: Arc<AtomicBool>,
    pub(crate) player_id: Arc<std::sync::OnceLock<String>>,
}

impl OutputStream {
    pub fn new(connection: Option<Arc<Connection>>) -> Self {
        Self {
            connection,
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

    fn send_datagram(&self, connection: &Connection, payload: Bytes) -> DatagramResult {
        let send_res = connection.datagram_mut(
            |dg: &mut common::s2n_quic::provider::datagram::default::Sender| {
                dg.send_datagram(payload)
            },
        );

        match send_res {
            Ok(Ok(())) => DatagramResult::Ok,
            Ok(Err(e)) => {
                let emsg = e.to_string();
                let lower = emsg.to_ascii_lowercase();
                if (lower.contains("connection") && lower.contains("clos"))
                    || lower.contains("closed")
                    || lower.contains("reset")
                {
                    DatagramResult::ConnectionClosed(emsg)
                } else if lower.contains("capacity") || lower.contains("queue") {
                    DatagramResult::Capacity(emsg)
                } else {
                    DatagramResult::Other(emsg)
                }
            }
            Err(e) => DatagramResult::Fatal(e.to_string()),
        }
    }
}

enum DatagramResult {
    Ok,
    ConnectionClosed(String),
    Capacity(String),
    Other(String),
    Fatal(String),
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
        tracing::info!("Starting QUIC output stream");
        self.is_stopped.store(false, Ordering::Relaxed);

        if let (Some(connection), Some(mut packet_rx)) =
            (self.connection.clone(), self.packet_rx.take())
        {
            while let Some(routed) = packet_rx.recv().await {
                let payload = match routed {
                    RoutedPacket::Serialized(bytes) => bytes,
                };

                let player = self.get_player_id().unwrap_or_else(|| "unknown".into());

                match self.send_datagram(&connection, payload) {
                    DatagramResult::Ok => {}
                    DatagramResult::ConnectionClosed(emsg) => {
                        tracing::error!(
                            "datagram_send_closed player={} err={}",
                            player,
                            emsg
                        );
                        break;
                    }
                    DatagramResult::Capacity(emsg) => {
                        tracing::debug!(
                            "datagram send capacity issue player={} err={}",
                            player,
                            emsg
                        );
                    }
                    DatagramResult::Other(emsg) => {
                        tracing::debug!(
                            "datagram send error player={} err={}",
                            player,
                            emsg
                        );
                    }
                    DatagramResult::Fatal(emsg) => {
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
