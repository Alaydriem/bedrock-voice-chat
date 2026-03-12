//! QUIC Server Manager
//!
//! Event-driven QUIC server implementation with ConnectionRegistry-based packet routing.
//!
//! ## Architecture:
//! - Each QUIC connection spawns a pair of InputStream/OutputStream
//! - InputStreams receive packets and route them via ConnectionRegistry
//! - AudioFrame packets are routed to specific recipients based on spatial/channel logic
//! - Non-audio packets (PlayerData, ChannelEvent, PlayerPresence) are broadcast to all
//! - CacheManager processes packets and updates coordinates for AudioFrame packets
//! - Graceful shutdown via oneshot channels

mod cache_manager;
pub(crate) mod connection_registry;
mod stream_manager;
mod webhook_receiver;

use crate::config::ApplicationConfig;
use anyhow;
use common::structs::packet::{PacketType, QuicNetworkPacket};
use common::traits::StreamTrait;
use common::s2n_quic::Server;
use connection_registry::ConnectionRegistry;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use stream_manager::{InputStream, OutputStream};
use tokio::sync::{mpsc, oneshot};

/// Helper function to create a short hash representation of client_id
pub(crate) fn client_id_hash(client_id: &[u8]) -> String {
    let mut hasher = DefaultHasher::new();
    client_id.hash(&mut hasher);
    format!("{:x}", hasher.finish() & 0xFFFF)
}

pub use cache_manager::CacheManager;
pub use webhook_receiver::WebhookReceiver;

#[derive(Debug, Clone)]
pub struct ServerInputPacket {
    pub data: QuicNetworkPacket,
}

pub struct QuicServerManager {
    config: ApplicationConfig,
    connection_registry: Arc<ConnectionRegistry>,
    webhook_rx: Option<mpsc::UnboundedReceiver<QuicNetworkPacket>>,
    cache_manager: CacheManager,
    webhook_receiver: WebhookReceiver,
    shutdown_tx: Option<oneshot::Sender<()>>,
    shutdown_rx: Option<oneshot::Receiver<()>>,
}

impl QuicServerManager {
    pub fn new(config: ApplicationConfig) -> Self {
        let connection_registry = Arc::new(ConnectionRegistry::new());
        let (webhook_tx, webhook_rx) = mpsc::unbounded_channel();
        let (shutdown_tx, shutdown_rx) = oneshot::channel();

        let mut cache_manager = CacheManager::new();
        cache_manager.set_connection_registry(connection_registry.clone());

        let webhook_receiver = WebhookReceiver::new(webhook_tx.clone());

        Self {
            config: config.clone(),
            connection_registry,
            webhook_rx: Some(webhook_rx),
            cache_manager,
            webhook_receiver,
            shutdown_tx: Some(shutdown_tx),
            shutdown_rx: Some(shutdown_rx),
        }
    }

    pub async fn start(&mut self) -> Result<(), anyhow::Error> {
        tracing::info!("Starting QUIC server manager");

        let (ca_cert, ca_key) = self.get_certificates().await?;

        let provider = common::rustls::MtlsProvider::new_from_vec(
            ca_cert.as_bytes().to_vec(),
            ca_cert.as_bytes().to_vec(),
            ca_key.as_bytes().to_vec(),
        )
        .await?;

        let bind_addr = format!(
            "{}:{}",
            self.config.server.listen, self.config.server.quic_port
        );

        let dg_endpoint = {
            let send_cap = if self.config.voice.datagram_send_capacity == 0 {
                1024
            } else {
                self.config.voice.datagram_send_capacity
            };
            let recv_cap = if self.config.voice.datagram_recv_capacity == 0 {
                1024
            } else {
                self.config.voice.datagram_recv_capacity
            };
            let builder = common::s2n_quic::provider::datagram::default::Endpoint::builder()
                .with_send_capacity(send_cap)
                .expect("datagram send capacity must be > 0")
                .with_recv_capacity(recv_cap)
                .expect("datagram recv capacity must be > 0");
            builder.build().expect("datagram endpoint build")
        };

        let server = Server::builder()
            .with_event(common::s2n_quic::provider::event::tracing::Subscriber::default())?
            .with_tls(provider)?
            .with_io(bind_addr.as_str())?
            .with_datagram(dg_endpoint)?
            .start()?;

        let mut webhook_rx = self.webhook_rx.take()
            .ok_or_else(|| anyhow::anyhow!("QUIC server already started"))?;
        let cache_manager = self.cache_manager.clone();
        let connection_registry = self.connection_registry.clone();
        let player_cache = cache_manager.get_player_cache();
        let broadcast_range = self.config.voice.spatial_audio.broadcast_range;
        let deafen_distance = self.config.voice.spatial_audio.deafen_distance;
        let mut shutdown_rx = self.shutdown_rx.take()
            .ok_or_else(|| anyhow::anyhow!("QUIC server already started"))?;

        tracing::info!("QUIC server started on {}", bind_addr);

        tokio::select! {
            _ = async {
                while let Some(packet) = webhook_rx.recv().await {
                    if let Err(e) = cache_manager.process_packet(packet.clone()).await {
                        tracing::error!("Failed to process packet in cache manager: {}", e);
                    }

                    match packet.packet_type {
                        PacketType::AudioFrame => {
                            connection_registry
                                .route_audio_frame(&packet, &player_cache, broadcast_range, deafen_distance)
                                .await;
                        }
                        _ => {
                            connection_registry.broadcast_to_all(packet);
                        }
                    }
                }
            } => {
                tracing::info!("Webhook processing completed");
            }

            _ = self.accept_connections(server) => {
                tracing::info!("QUIC connection handler completed");
            }

            _ = &mut shutdown_rx => {
                tracing::info!("Shutdown signal received");
            }
        }

        Ok(())
    }

    pub async fn stop(&mut self) -> Result<(), anyhow::Error> {
        tracing::info!("Stopping QUIC server");

        if let Some(shutdown_tx) = self.shutdown_tx.take() {
            let _ = shutdown_tx.send(());
        }

        tracing::info!("QUIC server stopped");
        Ok(())
    }

    pub fn get_cache_manager(&self) -> CacheManager {
        self.cache_manager.clone()
    }

    pub fn get_webhook_receiver(&self) -> &WebhookReceiver {
        &self.webhook_receiver
    }

    async fn get_certificates(&self) -> Result<(String, String), anyhow::Error> {
        let cert_path = format!("{}/ca.crt", self.config.server.tls.certs_path);
        let key_path = format!("{}/ca.key", self.config.server.tls.certs_path);

        if let (Ok(cert), Ok(key)) = (
            std::fs::read_to_string(&cert_path),
            std::fs::read_to_string(&key_path),
        ) {
            return Ok((cert, key));
        }

        Err(anyhow::anyhow!(
            "Certificates not found. Please generate certificates first."
        ))
    }

    async fn accept_connections(&self, mut server: Server) -> Result<(), anyhow::Error> {
        while let Some(mut connection) = server.accept().await {
            let connection_id = format!("{:?}", connection.id());
            tracing::info!("New QUIC connection accepted: {}", connection_id);

            let connection_registry = self.connection_registry.clone();
            let cache_manager = self.cache_manager.clone();
            let broadcast_range = self.config.voice.spatial_audio.broadcast_range;
            let deafen_distance = self.config.voice.spatial_audio.deafen_distance;
            let webhook_receiver = self.webhook_receiver.clone();

            tokio::spawn(async move {
                if let Err(e) = connection.keep_alive(true) {
                    tracing::warn!("Keepalive failed {}: {}", connection_id, e);
                }
                let conn_arc = Arc::new(connection);

                // Create per-connection mpsc channel for routed packets
                let (packet_tx, packet_rx) =
                    mpsc::channel::<connection_registry::RoutedPacket>(500);

                let mut input_stream = InputStream::new(Some(conn_arc.clone()), None);
                let mut output_stream = OutputStream::new(Some(conn_arc.clone()));
                output_stream.set_packet_receiver(packet_rx);

                // Identity callback: set output stream identity + register in connection registry
                let output_stream_identity_setter = {
                    let player_id_lock = output_stream.player_id.clone();
                    let client_id_lock = output_stream.client_id.clone();
                    let registry = connection_registry.clone();
                    let tx = packet_tx.clone();
                    move |player_id: String, client_id: Vec<u8>| {
                        if player_id_lock.set(player_id.clone()).is_err() {
                            tracing::warn!("Player ID already set for connection");
                        }
                        if client_id_lock.set(client_id.clone()).is_err() {
                            tracing::warn!("Client ID already set for connection");
                        }
                        registry.register(client_id, player_id, tx.clone());
                    }
                };

                // Disconnect callback: unregister from registry + cache cleanup
                let cache_manager_for_callback = cache_manager.clone();
                let webhook_receiver_for_callback = webhook_receiver.clone();
                let registry_for_callback = connection_registry.clone();
                input_stream.set_disconnect_callback(Box::new(
                    move |player_id: String, client_id: Vec<u8>| {
                        let cache_manager = cache_manager_for_callback.clone();
                        let webhook_receiver = webhook_receiver_for_callback.clone();
                        let registry = registry_for_callback.clone();
                        tokio::spawn(async move {
                            let client_hash = client_id_hash(&client_id);
                            tracing::info!(
                                "Player {} (client: {}) disconnected",
                                player_id,
                                client_hash
                            );

                            registry.unregister(&client_id);

                            match cache_manager.remove_player(&player_id).await {
                                Ok(removed_channels) => {
                                    for channel_id in removed_channels {
                                        let leave_packet = common::structs::packet::QuicNetworkPacket {
                                            owner: Some(common::structs::packet::PacketOwner {
                                                name: player_id.clone(),
                                                client_id: client_id.clone(),
                                            }),
                                            packet_type: common::structs::packet::PacketType::ChannelEvent,
                                            data: common::structs::packet::QuicNetworkPacketData::ChannelEvent(
                                                common::structs::packet::ChannelEventPacket::new(
                                                    common::structs::channel::ChannelEvents::Leave,
                                                    player_id.clone(),
                                                    channel_id.clone(),
                                                ),
                                            ),
                                        };

                                        if let Err(e) = webhook_receiver.send_packet(leave_packet).await {
                                            tracing::error!(
                                                "Failed to broadcast channel leave event for player {} channel {}: {}",
                                                player_id,
                                                channel_id,
                                                e
                                            );
                                        } else {
                                            tracing::info!(
                                                "Broadcast channel leave event: player {} left channel {}",
                                                player_id,
                                                channel_id
                                            );
                                        }
                                    }
                                }
                                Err(e) => {
                                    tracing::error!("Failed to remove player {}: {}", player_id, e);
                                }
                            }
                        });
                    },
                ));

                input_stream.set_webhook_receiver(webhook_receiver.clone());

                let (input_shutdown_tx, input_shutdown_rx) = oneshot::channel();
                let (output_shutdown_tx, output_shutdown_rx) = oneshot::channel();

                let input_registry = connection_registry.clone();
                let input_cache_manager = cache_manager.clone();
                let input_task = tokio::spawn(async move {
                    if let Err(e) = Self::run_input_stream_with_player_callback(
                        input_stream,
                        input_registry,
                        input_cache_manager,
                        broadcast_range,
                        deafen_distance,
                        input_shutdown_rx,
                        Box::new(output_stream_identity_setter),
                    )
                    .await
                    {
                        tracing::error!("Input stream error: {}", e);
                    }
                });

                let output_task = tokio::spawn(async move {
                    if let Err(e) = Self::run_output_stream(output_stream, output_shutdown_rx).await
                    {
                        tracing::error!("Output stream error: {}", e);
                    }
                });

                tokio::select! {
                    _ = input_task => { let _ = output_shutdown_tx.send(()); },
                    _ = output_task => { let _ = input_shutdown_tx.send(()); }
                }

                tracing::info!("Connection {} closed", connection_id);
            });
        }
        Ok(())
    }

    async fn run_input_stream_with_player_callback(
        mut input_stream: InputStream,
        connection_registry: Arc<ConnectionRegistry>,
        cache_manager: CacheManager,
        broadcast_range: f32,
        deafen_distance: f32,
        mut shutdown_rx: oneshot::Receiver<()>,
        player_callback: Box<dyn Fn(String, Vec<u8>) + Send + Sync>,
    ) -> Result<(), anyhow::Error> {
        let (packet_tx, mut packet_rx) = mpsc::unbounded_channel();
        input_stream.set_producer(packet_tx);

        let stream_task = tokio::spawn(async move { input_stream.start().await });

        let player_cache = cache_manager.get_player_cache();
        let mut has_set_identity = false;

        loop {
            tokio::select! {
                Some(server_packet) = packet_rx.recv() => {
                    let packet = server_packet.data;

                    if !has_set_identity && packet.owner.is_some() {
                        let owner = packet.owner.as_ref().unwrap();
                        player_callback(owner.name.clone(), owner.client_id.clone());
                        has_set_identity = true;
                        tracing::info!("Notified output stream of player identity: {}", owner.name);
                    }

                    if let Err(e) = cache_manager.process_packet(packet.clone()).await {
                        tracing::error!("Failed to process packet in cache manager: {}", e);
                    }

                    let updated_packet = if packet.packet_type == PacketType::AudioFrame {
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

                    match updated_packet.packet_type {
                        PacketType::AudioFrame => {
                            connection_registry
                                .route_audio_frame(&updated_packet, &player_cache, broadcast_range, deafen_distance)
                                .await;
                        }
                        _ => {
                            connection_registry.broadcast_to_all(updated_packet);
                        }
                    }
                }
                _ = &mut shutdown_rx => {
                    tracing::info!("Input stream received shutdown signal");
                    break;
                }
            }
        }

        let _ = stream_task.await;

        Ok(())
    }

    async fn run_output_stream(
        mut output_stream: OutputStream,
        mut shutdown_rx: oneshot::Receiver<()>,
    ) -> Result<(), anyhow::Error> {
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
