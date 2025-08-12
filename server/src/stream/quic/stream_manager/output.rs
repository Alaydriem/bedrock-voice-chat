use common::traits::StreamTrait;
use common::structs::packet::QuicNetworkPacket;
use anyhow::Error;
use s2n_quic::stream::SendStream;
use tokio::sync::{mpsc, broadcast};
use std::sync::Arc;
use crate::stream::quic::ServerOutputPacket;
use std::sync::atomic::{AtomicBool, Ordering};
use async_mutex::Mutex;
use moka::future::Cache;
use common::Player;

pub(crate) struct OutputStream {
    sender: Option<SendStream>,
    // Direct broadcast receiver instead of mutex consumer
    broadcast_rx: Option<broadcast::Receiver<QuicNetworkPacket>>,
    is_stopped: Arc<AtomicBool>,
    // Player identity for filtering (shared with corresponding InputStream)
    pub(crate) player_id: Arc<std::sync::Mutex<Option<String>>>,
    // Caches needed for packet filtering
    channel_cache: Option<Arc<Mutex<Cache<String, String>>>>,
    player_cache: Option<Arc<Cache<String, Player>>>,
    broadcast_range: f32,
}

impl OutputStream {
    pub fn new(
        sender: Option<SendStream>,
        _consumer: Option<Arc<tokio::sync::Mutex<mpsc::UnboundedReceiver<ServerOutputPacket>>>>,
    ) -> Self {
        Self {
            sender,
            broadcast_rx: None,
            is_stopped: Arc::new(AtomicBool::new(true)),
            player_id: Arc::new(std::sync::Mutex::new(None)),
            channel_cache: None,
            player_cache: None,
            broadcast_range: 20.0, // Default value
        }
    }

    pub fn set_broadcast_receiver(&mut self, broadcast_rx: broadcast::Receiver<QuicNetworkPacket>) {
        self.broadcast_rx = Some(broadcast_rx);
    }

    pub fn set_caches(
        &mut self,
        channel_cache: Arc<Cache<String, String>>,
        player_cache: Arc<Cache<String, Player>>,
        broadcast_range: f32,
    ) {
        // The packet.is_receivable method expects Arc<Mutex<Cache>>, but our cache manager
        // provides Arc<Cache>. We need to clone the cache content and wrap it.
        // Since Cache is already thread-safe, this is redundant but required by the API.
        self.channel_cache = Some(Arc::new(Mutex::new((*channel_cache).clone())));
        self.player_cache = Some(player_cache);
        self.broadcast_range = broadcast_range;
    }

    #[allow(unused)]
    pub fn set_sender(&mut self, sender: SendStream) {
        self.sender = Some(sender);
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

    /// Check if this output stream should receive a packet based on player identity
    /// This is a synchronous wrapper that will need to be called from async context
    pub async fn is_receivable(
        &self,
        packet: &mut QuicNetworkPacket,
    ) -> bool {
        // If we don't have a player ID yet, we can't filter
        let player_id = match self.get_player_id() {
            Some(id) => id,
            None => return true, // Pass through until we have identity
        };

        // If we don't have caches set up, use simple filtering
        let (channel_cache, player_cache) = match (&self.channel_cache, &self.player_cache) {
            (Some(cc), Some(pc)) => (cc.clone(), pc.clone()),
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
        packet.is_receivable(recipient, channel_cache, player_cache, self.broadcast_range).await
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
        
        if let (Some(mut sender), Some(mut broadcast_rx)) = (self.sender.take(), self.broadcast_rx.take()) {
            // Handle outgoing packets to this connection directly from broadcast
            loop {
                match broadcast_rx.recv().await {
                    Ok(mut packet) => {
                        // Check if this stream should receive this packet
                        if !self.is_receivable(&mut packet).await {
                            continue; // Skip packets not intended for this player
                        }

                        match packet.to_vec() {
                            Ok(data) => {
                                // Using sender.send() for single packet delivery
                                // Note: write_all() would be needed for:
                                // - Streaming large data that exceeds QUIC frame size
                                // - When you need to guarantee all bytes are written
                                // - For protocols requiring specific byte ordering
                                // Current use case: Single voice packets fit in QUIC frames
                                if let Err(e) = sender.send(data.into()).await {
                                    tracing::error!("Failed to send data over QUIC stream: {}", e);
                                    break;
                                }
                            }
                            Err(e) => {
                                tracing::error!("Failed to serialize packet: {}", e);
                                continue;
                            }
                        }
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        tracing::info!("Broadcast channel closed");
                        break;
                    }
                    Err(broadcast::error::RecvError::Lagged(skipped)) => {
                        tracing::warn!("Output stream lagged, skipped {} messages", skipped);
                        continue;
                    }
                }
            }
        }
        
        self.is_stopped.store(true, Ordering::Relaxed);
        Ok(())
    }

    async fn metadata(&mut self, key: String, value: String) -> Result<(), Error> {
        tracing::info!("Setting metadata for QUIC output stream: {} = {}", key, value);
        Ok(())
    }
}