use crate::stream::quic::client_id_hash;
use anyhow::Error;
use bytes::Bytes;
use common::structs::packet::QuicNetworkPacket;
use common::traits::StreamTrait;
use common::Player;
use moka::future::Cache;
use s2n_quic::Connection;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::broadcast;

pub(crate) struct OutputStream {
    connection: Option<Arc<Connection>>,
    // Direct broadcast receiver instead of mutex consumer
    broadcast_rx: Option<broadcast::Receiver<QuicNetworkPacket>>,
    is_stopped: Arc<AtomicBool>,
    // Player identity for filtering (shared with corresponding InputStream)
    pub(crate) player_id: Arc<std::sync::Mutex<Option<String>>>,
    // Client id for enriched logging
    pub(crate) client_id: Arc<std::sync::Mutex<Option<Vec<u8>>>>,
    // Caches needed for packet filtering
    channel_membership: Option<Arc<Cache<String, std::collections::HashSet<String>>>>,
    player_cache: Option<Arc<Cache<String, Player>>>,
    broadcast_range: f32,
}

impl OutputStream {
    pub fn new(connection: Option<Arc<Connection>>) -> Self {
        Self {
            connection,
            broadcast_rx: None,
            is_stopped: Arc::new(AtomicBool::new(true)),
            player_id: Arc::new(std::sync::Mutex::new(None)),
            client_id: Arc::new(std::sync::Mutex::new(None)),
            channel_membership: None,
            player_cache: None,
            broadcast_range: 20.0, // Default value
        }
    }

    pub fn set_broadcast_receiver(&mut self, broadcast_rx: broadcast::Receiver<QuicNetworkPacket>) {
        self.broadcast_rx = Some(broadcast_rx);
    }

    pub fn set_caches(
        &mut self,
        channel_membership: Arc<Cache<String, std::collections::HashSet<String>>>,
        player_cache: Arc<Cache<String, Player>>,
        broadcast_range: f32,
    ) {
        // Set up caches for packet filtering
        self.channel_membership = Some(channel_membership);
        self.player_cache = Some(player_cache);
        self.broadcast_range = broadcast_range;
    }

    #[allow(unused)]
    pub fn set_connection(&mut self, connection: Arc<Connection>) {
        self.connection = Some(connection);
    }

    #[allow(unused)]
    pub fn set_player_id(&self, player_id: String) {
        if let Ok(mut guard) = self.player_id.lock() {
            *guard = Some(player_id);
        }
    }

    pub fn get_player_id(&self) -> Option<String> {
        self.player_id.lock().ok().and_then(|guard| guard.clone())
    }

    #[allow(unused)]
    pub fn set_client_id(&self, client_id: Vec<u8>) {
        if let Ok(mut guard) = self.client_id.lock() {
            *guard = Some(client_id);
        }
    }

    pub fn get_client_id(&self) -> Option<Vec<u8>> {
        self.client_id.lock().ok().and_then(|guard| guard.clone())
    }

    /// Check if this output stream should receive a packet based on player identity
    /// This is a synchronous wrapper that will need to be called from async context
    pub async fn is_receivable(&self, packet: &mut QuicNetworkPacket) -> bool {
        // If we don't have a player ID yet, we can't filter
        let player_id = match self.get_player_id() {
            Some(id) => id,
            None => return true, // Pass through until we have identity
        };

        // If we don't have caches set up, use simple filtering
        let (channel_membership, player_cache) = match (&self.channel_membership, &self.player_cache) {
            (Some(cm), Some(pc)) => (cm.clone(), pc.clone()),
            _ => {
                // Fallback to simple self-filtering if caches aren't available
                match &packet.owner {
                    Some(owner) => return owner.name != player_id,
                    None => return true,
                }
            }
        };

        // Create recipient PacketOwner for this output stream's player
        let recipient = common::structs::packet::PacketOwner {
            name: player_id,
            client_id: vec![], // Client ID not needed for filtering
        };

        // Use the existing packet filtering logic
        packet
            .is_receivable(recipient, channel_membership, player_cache, self.broadcast_range)
            .await
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
        tracing::info!("Starting QUIC output stream");
        self.is_stopped.store(false, Ordering::Relaxed);

        if let (Some(connection), Some(mut broadcast_rx)) =
            (self.connection.clone(), self.broadcast_rx.take())
        {
            // Handle outgoing packets to this connection directly from broadcast
            while let Ok(mut packet) = broadcast_rx.recv().await {
                // Check if this stream should receive this packet
                if !self.is_receivable(&mut packet).await {
                    continue; // Skip packets not intended for this player
                }
                match packet.to_datagram() {
                    Ok(bytes) => {
                        let payload = Bytes::from(bytes);
                        let send_res = connection.datagram_mut(
                            |dg: &mut s2n_quic::provider::datagram::default::Sender| {
                                dg.send_datagram(payload.clone())
                            },
                        );
                        // Identity for logs
                        let player = self.get_player_id().unwrap_or_else(|| "unknown".into());
                        let client_hash = self
                            .get_client_id()
                            .map(|cid| client_id_hash(&cid))
                            .unwrap_or_else(|| "????".into());

                        fn is_conn_closed(msg: &str) -> bool {
                            let m = msg.to_ascii_lowercase();
                            (m.contains("connection") && m.contains("clos"))
                                || m.contains("closed")
                                || m.contains("reset")
                        }

                        match send_res {
                            Ok(Ok(())) => { /* sent */ }
                            Ok(Err(e)) => {
                                let emsg = e.to_string();
                                if is_conn_closed(&emsg) {
                                    tracing::error!(
                                        "datagram_send_closed player={} client={} err={}",
                                        player,
                                        client_hash,
                                        emsg
                                    );
                                    break;
                                } else if emsg.to_ascii_lowercase().contains("capacity")
                                    || emsg.to_ascii_lowercase().contains("queue")
                                {
                                    tracing::debug!(
                                        "datagram send capacity issue player={} client={} err={}",
                                        player,
                                        client_hash,
                                        emsg
                                    );
                                } else {
                                    tracing::debug!(
                                        "datagram send error player={} client={} err={}",
                                        player,
                                        client_hash,
                                        emsg
                                    );
                                }
                            }
                            Err(e) => {
                                let emsg = e.to_string();
                                tracing::error!(
                                    "datagram_send_query_failed player={} client={} err={}",
                                    player,
                                    client_hash,
                                    emsg
                                );
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to serialize packet to datagram: {}", e);
                        continue;
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
