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
mod certificate_common_name;
mod client_id_hasher;
mod connection_id_format;
mod connection_identity;
pub mod connection_registry;
mod packet_identity_stamp;
mod peer_identity_capture;
mod peer_identity_context;
mod server_input_packet;
mod stream_manager;
mod webhook_receiver;

use crate::config::ApplicationConfig;
use anyhow;
use client_id_hasher::ClientIdHasher;
use common::s2n_quic::{Connection, Server};
use common::structs::network::QuicCloseCode;
use common::structs::packet::{
    PacketType, PlayerDataPacket, PlayerPositionPacket, QuicNetworkPacket, QuicNetworkPacketData,
};
use common::traits::StreamTrait;
use connection_registry::ConnectionRegistry;
use std::sync::Arc;
use stream_manager::{InputStream, OutputStream};
use tokio::sync::{mpsc, oneshot};

pub use cache_manager::{CacheManager, CacheTrait, PlayerPreferenceCache, PlayerStateCache};
pub use certificate_common_name::CertificateCommonName;
pub use connection_id_format::PrefixedConnectionIdFormat;
pub use connection_identity::{ConnectionClassifier, ConnectionKind};
pub use packet_identity_stamp::PacketIdentityStamp;
pub use peer_identity_capture::PeerIdentityCapture;
pub use peer_identity_context::PeerIdentityContext;
pub use server_input_packet::ServerInputPacket;
pub use webhook_receiver::WebhookReceiver;

pub struct QuicServerManager {
    config: ApplicationConfig,
    connection_registry: Arc<ConnectionRegistry>,
    webhook_rx: Option<mpsc::UnboundedReceiver<QuicNetworkPacket>>,
    cache_manager: CacheManager,
    webhook_receiver: WebhookReceiver,
    shutdown_tx: Option<oneshot::Sender<()>>,
    shutdown_rx: Option<oneshot::Receiver<()>>,
    readiness: Option<Arc<crate::runtime::ReadinessState>>,
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
            readiness: None,
        }
    }

    /// Installs the shared readiness flag the /health/readiness route reads.
    /// The flag is raised once the QUIC listener is accepting and lowered on
    /// any exit of the accept loop.
    pub fn set_readiness(&mut self, readiness: Arc<crate::runtime::ReadinessState>) {
        self.readiness = Some(readiness);
    }

}

impl StreamTrait for QuicServerManager {
    /// Stopped means `stop()` consumed the shutdown sender; before the first
    /// start the manager is idle, not stopped.
    fn is_stopped(&self) -> bool {
        self.shutdown_tx.is_none()
    }

    async fn metadata(&mut self, _key: String, _value: String) -> Result<(), anyhow::Error> {
        Ok(())
    }

    async fn start(&mut self) -> Result<(), anyhow::Error> {
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

        let builder = Server::builder()
            .with_event((
                common::s2n_quic::provider::event::tracing::Subscriber::default(),
                PeerIdentityCapture::default(),
            ))?
            .with_tls(provider)?
            .with_io(bind_addr.as_str())?
            .with_datagram(dg_endpoint)?;

        let server = if let Some(instance_id) =
            self.config.server.meridian.as_ref().map(|m| m.instance_id)
        {
            tracing::info!(instance_id, "Using prefixed connection ID format");
            builder
                .with_connection_id(PrefixedConnectionIdFormat::new(instance_id))?
                .start()?
        } else {
            builder.start()?
        };

        let mut webhook_rx = self
            .webhook_rx
            .take()
            .ok_or_else(|| anyhow::anyhow!("QUIC server already started"))?;
        let cache_manager = self.cache_manager.clone();
        let connection_registry = self.connection_registry.clone();
        let player_cache = cache_manager.players().inner_arc();
        let broadcast_range = self.config.voice.spatial_audio.broadcast_range;
        let deafen_distance = self.config.voice.spatial_audio.deafen_distance;
        let mut shutdown_rx = self
            .shutdown_rx
            .take()
            .ok_or_else(|| anyhow::anyhow!("QUIC server already started"))?;

        tracing::info!("QUIC server started on {}", bind_addr);

        if let Some(readiness) = &self.readiness {
            readiness.set_quic_ready(true);
        }

        tokio::select! {
            _ = async {
                while let Some(packet) = webhook_rx.recv().await {
                    // process_packet has no AudioFrame arm; skipping it avoids a
                    // full packet clone (audio payload included) per frame.
                    if packet.packet_type != PacketType::AudioFrame {
                        // Server-injected: no certificate, so no authenticated game.
                        if let Err(e) = cache_manager.process_packet(packet.clone(), None).await {
                            tracing::error!("Failed to process packet in cache manager: {}", e);
                        }
                    }

                    match packet.packet_type {
                        PacketType::AudioFrame => {
                            connection_registry.forward_local_to_peers(&packet);
                            connection_registry
                                .route_audio_frame(&packet, &player_cache, broadcast_range, deafen_distance)
                                .await;
                        }
                        PacketType::QueryState
                        | PacketType::PlayerPreference
                        | PacketType::ClientAction => {
                            // All three are consumed by cache_manager's
                            // process_packet (QueryState/PlayerPreference are
                            // cached; serverbound group ClientActions route
                            // through ClientActionService). None are broadcast.
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

        if let Some(readiness) = &self.readiness {
            readiness.set_quic_ready(false);
        }

        Ok(())
    }

    async fn stop(&mut self) -> Result<(), anyhow::Error> {
        tracing::info!("Stopping QUIC server");

        if let Some(readiness) = &self.readiness {
            readiness.set_quic_ready(false);
        }

        if let Some(shutdown_tx) = self.shutdown_tx.take() {
            let _ = shutdown_tx.send(());
        }

        tracing::info!("QUIC server stopped");
        Ok(())
    }
}

impl QuicServerManager {
    pub fn get_cache_manager(&self) -> CacheManager {
        self.cache_manager.clone()
    }

    // Shared connection registry, so the runtime can install the cross-server
    // relay `PeerManager` on it after construction (`set_peer_manager`).
    pub(crate) fn get_connection_registry(&self) -> Arc<ConnectionRegistry> {
        self.connection_registry.clone()
    }

    pub fn set_bedrock_event_service(
        &mut self,
        service: Arc<crate::services::BedrockEventService>,
    ) {
        self.cache_manager.set_bedrock_event_service(service);
    }

    /// Wires the fan-out sender for serverbound group ClientActions (the no-net
    /// control path); without it they are logged and dropped.
    pub fn set_control_webhook_receiver(&mut self, webhook: WebhookReceiver) {
        self.cache_manager.set_webhook_receiver(webhook);
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

    // The mTLS-verified Common Name for an accepted connection, captured during the
    // handshake by `PeerIdentityCapture`. `None` means no authenticated identity is
    // available, which the accept loop treats as a refusal.
    fn authenticated_cn(connection: &Connection) -> Option<String> {
        connection
            .query_event_context(|ctx: &PeerIdentityContext| ctx.cn())
            .ok()
            .flatten()
    }

    // The close code sent to a connection whose identity cannot be trusted. The
    // client keys off this value to stop reconnecting instead of retrying forever.
    fn unauthorized_code() -> common::s2n_quic::application::Error {
        common::s2n_quic::application::Error::new(QuicCloseCode::Unauthorized.as_u64())
            .unwrap_or(common::s2n_quic::application::Error::UNKNOWN)
    }

    async fn accept_connections(&self, mut server: Server) -> Result<(), anyhow::Error> {
        while let Some(mut connection) = server.accept().await {
            let connection_id = format!("{:?}", connection.id());
            tracing::info!("New QUIC connection accepted: {}", connection_id);

            // Routing and trust are anchored to the mTLS certificate, never to the
            // self-asserted `owner.name` on the wire. A connection with no readable
            // certificate identity is refused outright rather than guessed at.
            let authenticated_cn = match Self::authenticated_cn(&connection) {
                Some(cn) => cn,
                None => {
                    tracing::error!(
                        connection = %connection_id,
                        "Refusing connection: no mTLS identity could be read from the peer certificate"
                    );
                    connection.close(Self::unauthorized_code());
                    continue;
                }
            };

            let (player_identity, peer_endpoint) = match ConnectionClassifier::classify(
                &authenticated_cn,
            ) {
                ConnectionKind::Player { game, name } => (Some((game, name)), None),
                ConnectionKind::Peer { endpoint, .. } => (None, Some(endpoint)),
                ConnectionKind::Rejected { identity } => {
                    tracing::warn!(
                        connection = %connection_id,
                        identity = %identity,
                        "Refusing connection: certificate identity is not a valid player or peer CN"
                    );
                    connection.close(Self::unauthorized_code());
                    continue;
                }
            };

            tracing::info!(
                connection = %connection_id,
                identity = %authenticated_cn,
                "Connection authenticated"
            );

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
                    move |player_id: String, client_id: Vec<u8>, game: common::Game| {
                        if player_id_lock.set(player_id.clone()).is_err() {
                            tracing::warn!("Player ID already set for connection");
                        }
                        if client_id_lock.set(client_id.clone()).is_err() {
                            tracing::warn!("Client ID already set for connection");
                        }
                        registry.register(client_id, player_id, game, tx.clone());
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
                            let client_hash = ClientIdHasher::hash(&client_id);
                            tracing::info!(
                                "Player {} (client: {}) disconnected",
                                player_id,
                                client_hash
                            );

                            registry.unregister(&client_id);

                            match cache_manager.remove_player(&player_id, None).await {
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
                // Handle to the accepted connection, so a peer (acceptor) link can
                // spawn an outbound write pump that sends relayed datagrams BACK on
                // this same connection, which makes the relay bidirectional.
                let input_conn = conn_arc.clone();
                let input_task = tokio::spawn(async move {
                    if let Err(e) = Self::run_input_stream_with_player_callback(
                        input_stream,
                        input_registry,
                        input_cache_manager,
                        broadcast_range,
                        deafen_distance,
                        input_shutdown_rx,
                        Box::new(output_stream_identity_setter),
                        input_conn,
                        player_identity,
                        peer_endpoint,
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
        player_callback: Box<dyn Fn(String, Vec<u8>, common::Game) + Send + Sync>,
        connection: Arc<Connection>,
        player_identity: Option<(common::Game, String)>,
        peer_endpoint: Option<String>,
    ) -> Result<(), anyhow::Error> {
        let (packet_tx, mut packet_rx) = mpsc::unbounded_channel();
        input_stream.set_producer(packet_tx);

        let stream_task = tokio::spawn(async move { input_stream.start().await });

        let player_cache = cache_manager.players().inner_arc();
        // Identity is settled by the mTLS certificate before this loop starts. A
        // player's every inbound packet is stamped with its authenticated name; a
        // peer server feeds the relay ingest and is never stamped, because relayed
        // packets carry their original sender's identity single-hop.
        let authenticated_game = player_identity.as_ref().map(|(game, _)| game.clone());
        let mut registered = false;

        // An inbound peer link registers with the relay up front rather than waiting
        // for a first packet to reveal who it is, then drains its outbound queue back
        // onto this same connection so `forward_local`'s per-peer enqueues reach
        // acceptor-accepted peers. Mirrors the dialer's write pump.
        if let Some(endpoint) = &peer_endpoint {
            match connection_registry.peer_manager() {
                Some(pm) => {
                    pm.register_inbound(endpoint, std::time::Instant::now());

                    if let Some(mut outbound_rx) = pm.take_outbound_receiver(endpoint) {
                        let write_conn = connection.clone();
                        tokio::spawn(async move {
                            while let Some(relayed) = outbound_rx.recv().await {
                                if let Ok(bytes) = relayed.packet.to_datagram() {
                                    let _ = write_conn.datagram_mut(
                                        |dg: &mut common::s2n_quic::provider::datagram::default::Sender| {
                                            dg.send_datagram(bytes.into())
                                        },
                                    );
                                }
                            }
                        });
                    }

                    tracing::info!(
                        "Accepted inbound peer connection: {} (relay ingest path)",
                        endpoint
                    );
                }
                None => {
                    tracing::warn!(
                        "Inbound peer-identity connection {} but no relay manager is wired; dropping",
                        endpoint
                    );
                }
            }
        }

        loop {
            tokio::select! {
                Some(server_packet) = packet_rx.recv() => {
                    let mut packet = server_packet.data;

                    // The wire owner is a claim; overwrite it with the certificate
                    // identity before anything downstream reads it. Registration
                    // still waits for the first owned packet, because the per-device
                    // client_id it routes on only exists on the wire.
                    if let Some((game, name)) = &player_identity {
                        PacketIdentityStamp::apply(&mut packet, name);

                        if !registered {
                            if let Some(owner) = &packet.owner {
                                player_callback(
                                    name.clone(),
                                    owner.client_id.clone(),
                                    game.clone(),
                                );
                                tracing::info!(
                                    "Registered authenticated player identity: {name}"
                                );
                                registered = true;
                            }
                        }
                    }

                    // Inbound peer link: route every packet straight into the
                    // relay ingest (FromPeer) — single-hop, registration
                    // bypassed. Never touches the local client/broadcast path.
                    if let Some(endpoint) = &peer_endpoint {
                        if let Some(pm) = connection_registry.peer_manager() {
                            pm.ingest(endpoint, packet).await;
                        }
                        continue;
                    }

                    // process_packet has no AudioFrame arm; skipping it avoids a
                    // full packet clone (audio payload included) per frame.
                    if packet.packet_type != PacketType::AudioFrame {
                        if let Err(e) = cache_manager
                            .process_packet(packet.clone(), authenticated_game.clone())
                            .await
                        {
                            tracing::error!("Failed to process packet in cache manager: {}", e);
                        }
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
                            // Local-origin audio: forward to peer servers
                            // sharing the sender's relay world (single-hop;
                            // relayed-origin packets never reach this path).
                            connection_registry.forward_local_to_peers(&updated_packet);
                            connection_registry
                                .route_audio_frame(&updated_packet, &player_cache, broadcast_range, deafen_distance)
                                .await;
                        }
                        PacketType::PlayerPosition => {
                            if let QuicNetworkPacketData::PlayerPosition(PlayerPositionPacket {
                                player,
                            }) = updated_packet.data.clone()
                            {
                                let rebroadcast = QuicNetworkPacket {
                                    packet_type: PacketType::PlayerData,
                                    owner: updated_packet.owner.clone(),
                                    data: QuicNetworkPacketData::PlayerData(PlayerDataPacket {
                                        players: vec![player],
                                    }),
                                };
                                connection_registry.broadcast_to_all(rebroadcast);
                            }
                        }
                        PacketType::PeerPresenceObserved => {
                            // A local client reported a `!bvcp` code observed in the
                            // realm. Route it to the asker-side observe handler
                            // (Flow 1) to redeem against the offering minter and open
                            // the peer link. Never broadcast onward.
                            if let QuicNetworkPacketData::PeerPresenceObserved(observed) =
                                updated_packet.data
                            {
                                connection_registry.on_peer_presence_observed(observed.token);
                            }
                        }
                        PacketType::PeerAnnounceObserved => {
                            // A local client reported a peer `!bvca` announce observed
                            // in the realm. Record the peer endpoint for the observer's
                            // world so the offer/forward paths can reach it. Never
                            // broadcast onward.
                            if let QuicNetworkPacketData::PeerAnnounceObserved(announce) =
                                updated_packet.data
                            {
                                connection_registry
                                    .on_peer_announce_observed(announce.hashed_world, announce.endpoint);
                            }
                        }
                        PacketType::QueryState
                        | PacketType::PlayerPreference
                        | PacketType::ClientAction => {
                            // All three are consumed by cache_manager's
                            // process_packet (QueryState/PlayerPreference are
                            // cached; serverbound group ClientActions route
                            // through ClientActionService). None are broadcast.
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
