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
mod connection_id_format;
mod connection_identity;
pub mod connection_registry;
// Public so the integration crate can drive the one invariant this mechanism rests on: a sequence
// number is consumed only when a datagram is actually produced for a connection.
pub mod connection_sequence;
mod log_throttle;
mod packet_identity_stamp;
mod path_observer;
mod path_observer_context;
mod peer_identity_capture;
mod peer_identity_context;
mod server_input_packet;
mod stream_manager;
mod webhook_receiver;

use crate::config::ApplicationConfig;
use anyhow;
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

pub use cache_manager::{
    CacheManager, CacheTrait, PlayerPreferenceCache, PlayerStateCache, TicketIdentity,
    WebsocketTicketCache,
};
pub use certificate_common_name::CertificateCommonName;
pub use connection_id_format::PrefixedConnectionIdFormat;
pub use connection_identity::{ConnectionClassifier, ConnectionKind};
pub use packet_identity_stamp::PacketIdentityStamp;
pub use path_observer::PathObserver;
pub use path_observer_context::PathObserverContext;
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

    // Builds the endpoint for one bind address. The TLS provider and the datagram
    // endpoint are each consumed by the builder and neither is Clone, so every
    // attempt constructs its own rather than sharing.
    async fn build_endpoint(
        &self,
        bind_addr: &str,
        ca_cert: &str,
        ca_key: &str,
    ) -> Result<Server, anyhow::Error> {
        let provider = common::rustls::MtlsProvider::new_from_vec(
            ca_cert.as_bytes().to_vec(),
            ca_cert.as_bytes().to_vec(),
            ca_key.as_bytes().to_vec(),
        )
        .await?;

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

        // Defaults negotiate a 30s idle timeout and derive the keepalive from it at 3/4,
        // which puts a ping on the wire every 22.5s. Carrier translators routinely drop an
        // idle UDP mapping sooner than that; the mapping is then recreated on a new source
        // port, which the peer sees as a new path. s2n-quic allows five and reclaims none,
        // so the fifth rebinding silently drops every datagram that follows.
        //
        // Ten seconds sits under the shortest carrier timeouts observed in the field. It has
        // no effect unless `Connection::keep_alive` is enabled -- it is, below and on the
        // client -- so removing either call reverts this with nothing failing to say so.
        //
        // 45s must stay under Meridian's `connection_ttl` (60s default): if the proxy reaps
        // its socket while the connection is still live, the next datagram arrives from a new
        // address and costs one of the same five paths this exists to protect.
        let limits = common::s2n_quic::provider::limits::Limits::default()
            .with_max_keep_alive_period(std::time::Duration::from_secs(10))?
            .with_max_idle_timeout(std::time::Duration::from_secs(45))?;

        let builder = Server::builder()
            .with_event((
                (
                    common::s2n_quic::provider::event::tracing::Subscriber::default(),
                    PeerIdentityCapture::default(),
                ),
                PathObserver::default(),
            ))?
            .with_tls(provider)?
            .with_io(bind_addr)?
            .with_limits(limits)?
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

        Ok(server)
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

        let preferred = self.config.server.quic_bind_addr(self.config.server.quic_port);

        // A failed v6 bind means the host has no IPv6 stack. Falling back keeps
        // those installs serving IPv4 across an upgrade instead of failing to
        // start, at the cost of being unreachable for IPv6-only clients — which is
        // what the warning says out loud.
        let (server, bind_addr) = match self.build_endpoint(&preferred, &ca_cert, &ca_key).await {
            Ok(server) => (server, preferred),
            Err(e) => {
                let fallback = format!(
                    "{}:{}",
                    crate::config::Server::FALLBACK_LISTEN, self.config.server.quic_port
                );
                tracing::warn!(
                    "QUIC bind to {} failed ({}); falling back to {}. IPv6-only clients cannot reach this server.",
                    preferred,
                    e,
                    fallback
                );
                let server = self.build_endpoint(&fallback, &ca_cert, &ca_key).await?;
                (server, fallback)
            }
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
                        if let Err(e) = cache_manager.process_packet(packet.clone()).await {
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
                        PacketType::PlayerData => {
                            // Addressed to each player rather than broadcast:
                            // a client only ever reads its own entry.
                            connection_registry.send_positions_to_owners(&packet);
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

    pub fn set_chat_service(&mut self, service: Arc<crate::services::ChatService>) {
        self.cache_manager.set_chat_service(service);
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
            // The device id every outbound packet from this connection is stamped with, and
            // the registry's key. Taken from the connection rather than declared by the
            // client, which is what makes one player's two devices independently addressable
            // without either being able to impersonate the other.
            let device = connection.id();
            let connection_id = format!("{:?}", device);
            tracing::info!("New QUIC connection accepted: {}", connection_id);

            // Routing and trust are anchored to the mTLS certificate. A connection with no
            // readable certificate identity is refused outright rather than guessed at.
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
                // The canonical identity is carried, not the bare name: it is the key every
                // cache, the registry index and channel membership share.
                ConnectionKind::Player { game, name } => (Some(game.membership_key(&name)), None),
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
                if let Some(identity) = &player_identity {
                    input_stream.set_identity(identity.clone(), device);
                }
                let mut output_stream = OutputStream::new(Some(conn_arc.clone()));
                output_stream.set_packet_receiver(packet_rx);

                // Registers this connection under its authenticated identity. Both the
                // identity and the device id are known at accept, so this runs before the
                // first packet rather than being triggered by one.
                let register_connection = {
                    let player_id_lock = output_stream.player_id.clone();
                    let registry = connection_registry.clone();
                    let tx = packet_tx.clone();
                    move |identity: String| {
                        if player_id_lock.set(identity.clone()).is_err() {
                            tracing::warn!("Player ID already set for connection");
                        }
                        registry.register(device, identity, tx.clone());
                    }
                };

                // Disconnect callback: unregister from registry + cache cleanup
                let cache_manager_for_callback = cache_manager.clone();
                let webhook_receiver_for_callback = webhook_receiver.clone();
                let registry_for_callback = connection_registry.clone();
                input_stream.set_disconnect_callback(Box::new(
                    move |player_id: String| {
                        let cache_manager = cache_manager_for_callback.clone();
                        let webhook_receiver = webhook_receiver_for_callback.clone();
                        let registry = registry_for_callback.clone();
                        tokio::spawn(async move {
                            tracing::info!(
                                "Player {} (device: {}) disconnected",
                                player_id,
                                device
                            );

                            registry.unregister(device);

                            match cache_manager.remove_player(&player_id).await {
                                Ok(removed_channels) => {
                                    for channel_id in removed_channels {
                                        let leave_packet = common::structs::packet::QuicNetworkPacket {
                                            sender: Some(common::structs::packet::PacketSender::new(
                                                player_id.clone(),
                                                device,
                                            )),
                                            packet_type: common::structs::packet::PacketType::ChannelEvent,
                                            data: common::structs::packet::QuicNetworkPacketData::ChannelEvent(
                                                common::structs::packet::ChannelEventPacket::new(
                                                    common::structs::channel::ChannelEvents::Leave,
                                                    player_id.clone(),
                                                    channel_id.clone(),
                                                ),
                                            ),
                                                                                    // Not a server fan-out, so this envelope carries no sequence.
                                            ..Default::default()
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
                        Box::new(register_connection),
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
        register_connection: Box<dyn Fn(String) + Send + Sync>,
        connection: Arc<Connection>,
        player_identity: Option<String>,
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
        //
        // Read from the same connection this loop serves, so the stamped device id cannot
        // drift from the connection it names.
        let device = connection.id();

        // A player's connection is registered here, before its first packet. Both keys the
        // registry needs — the identity and the device id — come from the connection itself,
        // so nothing is waiting on the wire to reveal them.
        if let Some(identity) = &player_identity {
            register_connection(identity.clone());
            tracing::info!("Registered authenticated player identity: {identity}");
        }

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

                    // Stamp the certificate identity and this connection's device id before
                    // anything downstream reads either.
                    if let Some(identity) = &player_identity {
                        PacketIdentityStamp::apply(&mut packet, identity, device);
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
                            .process_packet(packet.clone())
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
                                // A proxy client self-reports its position and
                                // relies on this echo to anchor its own
                                // listener, so the packet still goes out -- but
                                // only to the player it describes.
                                let echo = QuicNetworkPacket {
                                    packet_type: PacketType::PlayerData,
                                    sender: updated_packet.sender.clone(),
                                    data: QuicNetworkPacketData::PlayerData(
                                        PlayerDataPacket::new(vec![player]),
                                    ),
                                    // Not a server fan-out, so this envelope carries no sequence.
                                    ..Default::default()
                                };
                                connection_registry.send_positions_to_owners(&echo);
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
                        PacketType::PlayerData => {
                            // Clients report position as PlayerPosition, so a
                            // clientbound-shaped PlayerData arriving here is not
                            // something the client should be sending. Address it
                            // per-player like every other position packet rather
                            // than letting one connection push coordinates to
                            // everyone.
                            connection_registry.send_positions_to_owners(&updated_packet);
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
