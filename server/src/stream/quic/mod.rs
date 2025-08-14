//! QUIC Server Manager
//! 
//! Event-driven QUIC server implementation with proper async/await patterns and broadcast channels.
//! 
//! ## Architecture:
//! - Each QUIC connection spawns a pair of InputStream/OutputStream
//! - InputStreams receive packets and broadcast them to all OutputStreams via broadcast channels
//! - OutputStreams filter packets based on player identity (no echo-back)
//! - CacheManager processes packets and updates coordinates for AudioFrame packets
//! - Graceful shutdown via oneshot channels
//! 
//! ## QUIC Protocol Features:
//! - Player identity extracted from first PlayerData packet
//! - QUIC disconnect signaling with error code 204 
//! - Coordinate updates for AudioFrame packets
//! - Bidirectional disconnect handling
//! - Callback system for cache cleanup on disconnect

mod cache_manager;
mod stream_manager;
mod webhook_receiver;

use crate::config::ApplicationConfig;
use common::structs::packet::QuicNetworkPacket;
use s2n_quic::Server;
use stream_manager::{InputStream, OutputStream};
use common::traits::StreamTrait;
use anyhow;
use tokio::sync::{broadcast, mpsc, oneshot};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Helper function to create a short hash representation of client_id
fn client_id_hash(client_id: &[u8]) -> String {
    let mut hasher = DefaultHasher::new();
    client_id.hash(&mut hasher);
    format!("{:x}", hasher.finish() & 0xFFFF) // Take only last 4 hex digits for readability
}

pub use cache_manager::CacheManager;
pub use webhook_receiver::WebhookReceiver;

// Define packet types similar to client
#[derive(Debug, Clone)]
pub struct ServerInputPacket {
    pub data: QuicNetworkPacket,
}

#[derive(Debug, Clone)]
pub struct ServerOutputPacket {
    pub data: QuicNetworkPacket,
}

pub struct QuicServerManager {
    config: ApplicationConfig,
    // Broadcast channel for distributing packets to all connected clients
    broadcast_tx: broadcast::Sender<QuicNetworkPacket>,
    // Channel for receiving packets from HTTP webhooks
    webhook_rx: Option<mpsc::UnboundedReceiver<QuicNetworkPacket>>,
    // Cache manager for player positions and channels
    cache_manager: CacheManager,
    // Webhook receiver for HTTP API integration
    webhook_receiver: WebhookReceiver,
    // Shutdown signal
    shutdown_tx: Option<oneshot::Sender<()>>,
    shutdown_rx: Option<oneshot::Receiver<()>>,
}

impl QuicServerManager {
    /// Creates a new QuicServerManager with the given application configuration
    pub fn new(config: ApplicationConfig) -> Self {
        let (broadcast_tx, _) = broadcast::channel(10000);
        let (webhook_tx, webhook_rx) = mpsc::unbounded_channel();
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        
        let cache_manager = CacheManager::new();
        
        let webhook_receiver = WebhookReceiver::new(
            webhook_tx.clone()
        );
        
        Self {
            config: config.clone(),
            broadcast_tx,
            webhook_rx: Some(webhook_rx),
            cache_manager,
            webhook_receiver,
            shutdown_tx: Some(shutdown_tx),
            shutdown_rx: Some(shutdown_rx),
        }
    }

    /// Main entry point - starts the QUIC server and runs until shutdown
    /// This should be the only blocking call that manages all connections
    pub async fn start(&mut self) -> Result<(), anyhow::Error> {
        tracing::info!("Starting QUIC server manager");

        // Generate/load certificates
        let (ca_cert, ca_key) = self.get_certificates().await?;
        
        // Setup TLS provider
        let provider = common::rustls::MtlsProvider::new_from_vec(
            ca_cert.as_bytes().to_vec(),
            ca_cert.as_bytes().to_vec(), // Using same cert for both CA and server
            ca_key.as_bytes().to_vec()
        ).await?;

        // Create bind address
        let bind_addr = format!("{}:{}", self.config.server.listen, self.config.server.quic_port);
        
        // Start QUIC server
        let server = Server::builder()
            .with_event(s2n_quic::provider::event::tracing::Subscriber::default())?
            .with_tls(provider)?
            .with_io(bind_addr.as_str())?
            .start()?;

        // Get webhook receiver and set up webhook processing
        let mut webhook_rx = self.webhook_rx.take().unwrap();
        let broadcast_tx = self.broadcast_tx.clone();
        let cache_manager = self.cache_manager.clone();
        let mut shutdown_rx = self.shutdown_rx.take().unwrap();

        tracing::info!("QUIC server started on {}", bind_addr);
        
        // Main event loop with proper async/await
        tokio::select! {
            // Handle webhook packets
            _ = async {
                while let Some(packet) = webhook_rx.recv().await {
                    // Process packet with cache manager
                    if let Err(e) = cache_manager.process_packet(packet.clone()).await {
                        tracing::error!("Failed to process packet in cache manager: {}", e);
                    }
                    
                    // Broadcast to all connected clients
                    if let Err(e) = broadcast_tx.send(packet) {
                        match e {
                            broadcast::error::SendError(_) => {
                                tracing::debug!("No active receivers for broadcast packet (no clients connected)");
                            }
                        }
                    }
                }
            } => {
                tracing::info!("Webhook processing completed");
            }
            
            // Handle QUIC connections
            _ = self.accept_connections(server) => {
                tracing::info!("QUIC connection handler completed");
            }
            
            // Wait for shutdown signal
            _ = &mut shutdown_rx => {
                tracing::info!("Shutdown signal received");
            }
        }
        
        Ok(())
    }

    /// Stops the QUIC server gracefully
    pub async fn stop(&mut self) -> Result<(), anyhow::Error> {
        tracing::info!("Stopping QUIC server");
        
        // Signal shutdown by sending to the oneshot channel
        if let Some(shutdown_tx) = self.shutdown_tx.take() {
            let _ = shutdown_tx.send(());
        }
        
        tracing::info!("QUIC server stopped");
        Ok(())
    }

    
    /// Gets the cache manager for shared access between components
    pub fn get_cache_manager(&self) -> CacheManager {
        self.cache_manager.clone()
    }

    /// Gets a reference to the webhook receiver for HTTP API integration
    pub fn get_webhook_receiver(&self) -> &WebhookReceiver {
        &self.webhook_receiver
    }

    /// Generates or loads certificates from the configuration
    async fn get_certificates(&self) -> Result<(String, String), anyhow::Error> {
        let cert_path = format!("{}/ca.crt", self.config.server.tls.certs_path);
        let key_path = format!("{}/ca.key", self.config.server.tls.certs_path);
        
        // Try to read existing certificates
        if let (Ok(cert), Ok(key)) = (
            std::fs::read_to_string(&cert_path),
            std::fs::read_to_string(&key_path)
        ) {
            return Ok((cert, key));
        }
        
        // If certificates don't exist, we should generate them
        // For now, return an error - certificate generation should be handled elsewhere
        Err(anyhow::anyhow!("Certificates not found. Please generate certificates first."))
    }

    /// Main connection acceptance loop
    /// Creates new InputStream/OutputStream pairs for each connection
    async fn accept_connections(&self, mut server: Server) -> Result<(), anyhow::Error> {
        while let Some(mut connection) = server.accept().await {
            let connection_id = format!("{:?}", connection.id());
            tracing::info!("New QUIC connection accepted: {}", connection_id);

            let broadcast_tx = self.broadcast_tx.clone();
            let cache_manager = self.cache_manager.clone();
            let broadcast_range = self.config.voice.broadcast_range;
            let webhook_receiver = self.webhook_receiver.clone(); // Clone outside of spawn

            // Spawn a task to handle this connection
            tokio::spawn(async move {
                // Enable keepalive for this connection to prevent idle timeouts
                if let Err(e) = connection.keep_alive(true) {
                    tracing::warn!("Failed to enable keepalive for connection {}: {}", connection_id, e);
                }
                
                // Accept a bidirectional stream for this connection
                match connection.accept_bidirectional_stream().await {
                    Ok(Some(stream)) => {
                        // Enable keepalive for the stream and its connection
                        if let Err(e) = stream.connection().keep_alive(true) {
                            tracing::warn!("Failed to enable stream connection keepalive: {}", e);
                        }
                        
                        let (receive_stream, send_stream) = stream.split();
                        
                        // Create shutdown channels for both streams
                        let (input_shutdown_tx, input_shutdown_rx) = oneshot::channel();
                        let (output_shutdown_tx, output_shutdown_rx) = oneshot::channel();
                        
                        // Create InputStream and OutputStream for this connection
                        let mut input_stream = InputStream::new(Some(receive_stream), None);
                        let mut output_stream = OutputStream::new(Some(send_stream), None);
                        
                        // Set up caches for packet filtering
                        output_stream.set_caches(
                            cache_manager.get_channel_cache(),
                            cache_manager.get_player_cache(),
                            broadcast_range,
                        );
                        
                        // Set up player identity sharing - create a callback that updates the output stream
                        let output_stream_player_setter = {
                            let player_id_mutex = output_stream.player_id.clone();
                            move |player_id: String| {
                                if let Ok(mut guard) = player_id_mutex.lock() {
                                    *guard = Some(player_id.clone());
                                    tracing::debug!("Set player identity for output stream: {}", player_id);
                                }
                            }
                        };
                        
                        // Set disconnect callback for cache cleanup
                        let cache_manager_for_callback = cache_manager.clone();
                        input_stream.set_disconnect_callback(Box::new(move |player_id: String, client_id: Vec<u8>| {
                            let cache_manager = cache_manager_for_callback.clone();
                            tokio::spawn(async move {
                                // Create short hash for logging
                                let client_hash = client_id_hash(&client_id);
                                
                                tracing::info!("Player {} (client: {}) disconnected, cleaning up cache", player_id, client_hash);
                                if let Err(e) = cache_manager.remove_player(&player_id).await {
                                    tracing::error!("Failed to remove player {} from cache: {}", player_id, e);
                                }
                            });
                        }));
                        
                        // Set webhook receiver for presence events
                        input_stream.set_webhook_receiver(webhook_receiver.clone());
                        
                        // Each output stream gets its own broadcast receiver
                        let broadcast_rx = broadcast_tx.subscribe();
                        output_stream.set_broadcast_receiver(broadcast_rx);
                        
                        // Start the input stream (handles incoming packets)
                        let input_broadcast_tx = broadcast_tx.clone();
                        let input_cache_manager = cache_manager.clone();
                        let input_task = tokio::spawn(async move {
                            if let Err(e) = Self::run_input_stream_with_player_callback(
                                input_stream, 
                                input_broadcast_tx, 
                                input_cache_manager,
                                input_shutdown_rx,
                                Box::new(output_stream_player_setter)
                            ).await {
                                tracing::error!("Input stream error: {}", e);
                            }
                        });
                        
                        // Start the output stream (handles outgoing packets)
                        let output_task = tokio::spawn(async move {
                            if let Err(e) = Self::run_output_stream(
                                output_stream,
                                output_shutdown_rx
                            ).await {
                                tracing::error!("Output stream error: {}", e);
                            }
                        });
                        
                        // Use select! instead of try_join! - if either ends, terminate both
                        tokio::select! {
                            result = input_task => {
                                tracing::info!("Input stream ended: {:?}", result);
                                // Signal output stream to shutdown
                                let _ = output_shutdown_tx.send(());
                            }
                            result = output_task => {
                                tracing::info!("Output stream ended: {:?}", result);
                                // Signal input stream to shutdown
                                let _ = input_shutdown_tx.send(());
                            }
                        }
                    }
                    Ok(None) => {
                        tracing::warn!("No bidirectional stream available for connection {}", connection_id);
                    }
                    Err(e) => {
                        tracing::error!("Failed to accept bidirectional stream for connection {}: {}", connection_id, e);
                    }
                }
                
                tracing::info!("Connection {} closed", connection_id);
            });
        }
        
        Ok(())
    }

    /// Run an input stream with player identity callback - handles incoming packets from a single connection
    async fn run_input_stream_with_player_callback(
        mut input_stream: InputStream,
        broadcast_tx: broadcast::Sender<QuicNetworkPacket>,
        cache_manager: CacheManager,
        mut shutdown_rx: oneshot::Receiver<()>,
        player_callback: Box<dyn Fn(String) + Send + Sync>,
    ) -> Result<(), anyhow::Error> {
        // Set up the input stream to broadcast received packets
        let (packet_tx, mut packet_rx) = mpsc::unbounded_channel();
        input_stream.set_producer(packet_tx);
        
        // Start the input stream
        let stream_task = tokio::spawn(async move {
            input_stream.start().await
        });
        
        let mut has_set_identity = false;
        
        // Process received packets with shutdown support
        loop {
            tokio::select! {
                Some(server_packet) = packet_rx.recv() => {
                    let packet = server_packet.data;
                    
                    // Extract player identity from first packet with owner and notify output stream
                    if !has_set_identity && packet.owner.is_some() {
                        let owner = packet.owner.as_ref().unwrap();
                        player_callback(owner.name.clone());
                        has_set_identity = true;
                        tracing::info!("Notified output stream of player identity: {}", owner.name);
                    }
                    
                    // Process packet with cache manager
                    if let Err(e) = cache_manager.process_packet(packet.clone()).await {
                        tracing::error!("Failed to process packet in cache manager: {}", e);
                    }
                    
                    // Update coordinates for AudioFrame packets
                    let updated_packet = if packet.packet_type == common::structs::packet::PacketType::AudioFrame {
                        match cache_manager.update_coordinates(packet).await {
                            Ok(updated) => updated,
                            Err(e) => {
                                tracing::error!("Failed to update coordinates: {}", e);
                                continue;
                            }
                        }
                    } else {
                        packet
                    };
                    
                    // Broadcast to all connected clients
                    if let Err(e) = broadcast_tx.send(updated_packet) {
                        tracing::error!("Failed to broadcast received packet: {}", e);
                        break;
                    }
                }
                _ = &mut shutdown_rx => {
                    tracing::info!("Input stream received shutdown signal");
                    break;
                }
            }
        }
        
        // Stop the stream
        let _ = stream_task.await;
        
        Ok(())
    }
    
    /// Run an output stream - handles outgoing packets to a single connection
    async fn run_output_stream(
        mut output_stream: OutputStream,
        mut shutdown_rx: oneshot::Receiver<()>,
    ) -> Result<(), anyhow::Error> {
        // Output stream now handles broadcast receiver directly, no more mpsc/mutex
        
        // Start the output stream with shutdown support
        tokio::select! {
            result = output_stream.start() => {
                if let Err(e) = result {
                    tracing::error!("Output stream error: {}", e);
                }
            }
            _ = &mut shutdown_rx => {
                tracing::info!("Output stream received shutdown signal");
                if let Err(e) = output_stream.stop().await {
                    tracing::error!("Error stopping output stream: {}", e);
                }
            }
        }
        
        Ok(())
    }
}
