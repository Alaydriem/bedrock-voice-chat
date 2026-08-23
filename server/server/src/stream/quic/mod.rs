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
pub mod connection;
mod connection_identity;
mod log_throttle;
mod packet_identity_stamp;
pub mod path;
pub mod peer;
mod server_input_packet;
pub mod stream_manager;
mod webhook_receiver;

use common::curia;
use crate::config::ApplicationConfig;
use crate::stream::session::SessionLink;
use anyhow;
use common::s2n_quic::{Connection, Server};
use common::structs::network::QuicCloseCode;
use common::structs::packet::{PacketType, QuicNetworkPacket};
use common::traits::StreamTrait;
use connection::ConnectionRegistry;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot};

pub use cache_manager::{
    CacheManager, CacheTrait, PlayerPreferenceCache, PlayerStateCache, TicketIdentity,
    WebsocketTicketCache,
};
pub use certificate_common_name::CertificateCommonName;
pub use connection::PrefixedConnectionIdFormat;
pub use connection_identity::{ConnectionClassifier, ConnectionKind};
pub use packet_identity_stamp::PacketIdentityStamp;
pub use path::{PathObserver, PathObserverContext};
pub use peer::identity::{PeerIdentityCapture, PeerIdentityContext};
pub use server_input_packet::ServerInputPacket;
pub use webhook_receiver::{PacketOrigin, WebhookReceiver};

pub struct QuicServerManager {
    config: ApplicationConfig,
    connection_registry: Arc<ConnectionRegistry>,
    webhook_rx: Option<mpsc::UnboundedReceiver<(QuicNetworkPacket, PacketOrigin)>>,
    cache_manager: CacheManager,
    webhook_receiver: WebhookReceiver,
    /// Serves every accepted session. Shared with the WebSocket listener, which is what
    /// keeps one routing implementation behind two transports.
    ///
    /// Built on first use, NOT in `new`. `CacheManager` carries its bedrock, chat and
    /// webhook services in plain `Option` fields that the runtime fills in with `&mut
    /// self` after this manager is constructed, and it is cloned by value. A copy taken
    /// in `new` is therefore permanently unwired -- packets still route, so nothing
    /// fails; the features hanging off those services just silently stop working.
    session_spawner: std::sync::OnceLock<Arc<crate::stream::session::SessionSpawner>>,
    shutdown_tx: Option<oneshot::Sender<()>>,
    shutdown_rx: Option<oneshot::Receiver<()>>,
    readiness: Option<Arc<crate::runtime::ReadinessState>>,
    /// Required, not an optional setter. There is one call site, so requiring it means
    /// the handshake cannot be left admitting revoked or banished certificates.
    authorization: Arc<crate::services::SessionAuthorizationService>,
    database: Arc<sea_orm::DatabaseConnection>,
}

impl QuicServerManager {
    pub fn new(
        config: ApplicationConfig,
        authorization: Arc<crate::services::SessionAuthorizationService>,
        database: Arc<sea_orm::DatabaseConnection>,
    ) -> Self {
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
            session_spawner: std::sync::OnceLock::new(),
            shutdown_tx: Some(shutdown_tx),
            shutdown_rx: Some(shutdown_rx),
            readiness: None,
            authorization,
            database,
        }
    }

    /// The session runner every accepted connection is served by.
    ///
    /// Handed to the WebSocket listener so both transports drive one implementation. It
    /// lives here because the registry and caches it closes over are built with this
    /// manager.
    pub(crate) fn session_spawner(&self) -> Arc<crate::stream::session::SessionSpawner> {
        self.session_spawner
            .get_or_init(|| {
                crate::stream::session::SessionSpawner::new_shared(
                    self.connection_registry.clone(),
                    self.cache_manager.clone(),
                    self.config.voice.spatial_audio.broadcast_range,
                    self.config.voice.spatial_audio.deafen_distance,
                    self.webhook_receiver.clone(),
                    self.config.voice.send_batch_wait_micros,
                )
            })
            .clone()
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
            curia::info!("Using prefixed connection ID format", { "instance_id": instance_id });
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
        curia::info!("Starting QUIC server manager");

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
                curia::warn!(
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

        curia::info!("QUIC server started on {}", bind_addr);

        if let Some(readiness) = &self.readiness {
            readiness.set_quic_ready(true);
        }

        tokio::select! {
            _ = async {
                while let Some((mut packet, origin)) = webhook_rx.recv().await {
                    // process_packet has no AudioFrame arm; skipping it avoids a
                    // full packet clone (audio payload included) per frame.
                    if packet.packet_type != PacketType::AudioFrame {
                        // Server-injected: no certificate, so no authenticated game.
                        if let Err(e) = cache_manager.process_packet(packet.clone()).await {
                            curia::error!("Failed to process packet in cache manager: {}", e);
                        }
                    }

                    match packet.packet_type {
                        PacketType::AudioFrame => {
                            // Resolved once and handed to both, so the relay and the fan-out
                            // cannot disagree about where a speaker is.
                            let speaker = cache_manager.resolve_speaker(&packet).await;
                            cache_manager.attach_speaker(&mut packet, speaker.as_ref());

                            // Only local audio goes back out to peers. Forwarding a
                            // peer's own frame returns it to the sender, who hears
                            // themselves; with two peers in one world it also loops
                            // between them without end.
                            if origin == PacketOrigin::Local {
                                connection_registry
                                    .forward_local_to_peers(&packet, speaker.as_ref());
                            }
                            connection_registry
                                .route_audio_frame(
                                    &packet,
                                    speaker.as_ref(),
                                    &player_cache,
                                    broadcast_range,
                                    deafen_distance,
                                )
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
                curia::info!("Webhook processing completed");
            }

            _ = self.accept_connections(server) => {
                curia::info!("QUIC connection handler completed");
            }

            _ = &mut shutdown_rx => {
                curia::info!("Shutdown signal received");
            }
        }

        if let Some(readiness) = &self.readiness {
            readiness.set_quic_ready(false);
        }

        Ok(())
    }

    async fn stop(&mut self) -> Result<(), anyhow::Error> {
        curia::info!("Stopping QUIC server");

        if let Some(readiness) = &self.readiness {
            readiness.set_quic_ready(false);
        }

        if let Some(shutdown_tx) = self.shutdown_tx.take() {
            let _ = shutdown_tx.send(());
        }

        curia::info!("QUIC server stopped");
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

    fn authenticated_leaf(connection: &Connection) -> Option<Vec<u8>> {
        connection
            .query_event_context(|ctx: &PeerIdentityContext| ctx.leaf_der())
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
            curia::info!("New QUIC connection accepted: {}", connection_id);

            // Routing and trust are anchored to the mTLS certificate. A connection with no
            // readable certificate identity is refused outright rather than guessed at.
            let authenticated_cn = match Self::authenticated_cn(&connection) {
                Some(cn) => cn,
                None => {
                    curia::error!("Refusing connection: no mTLS identity could be read from the peer certificate", { "connection": connection_id.to_string() });
                    connection.close(Self::unauthorized_code());
                    continue;
                }
            };

            // The certificate is authorized, not merely parsed. Before this, the CN was read
            // and trusted, so any certificate this CA had ever signed opened a voice session
            // whether or not the player still existed, was banished, or had been revoked.
            let Some(leaf_der) = Self::authenticated_leaf(&connection) else {
                curia::error!("Refusing connection: no peer certificate could be read", { "connection": connection_id.to_string() });
                connection.close(Self::unauthorized_code());
                continue;
            };

            let fingerprint =
                crate::services::SessionAuthorizationService::fingerprint(&leaf_der);

            let player = match self
                .authorization
                .authorize(self.database.as_ref(), &leaf_der)
                .await
            {
                Ok(player) => player,
                Err(reason) => {
                    curia::warn!(format!("Refusing connection: {}", reason), { "connection": connection_id.to_string(), "identity": authenticated_cn.to_string() });
                    connection.close(Self::unauthorized_code());
                    continue;
                }
            };

            // The canonical identity is carried, not the bare name: it is the key every
            // cache, the registry index and channel membership share. Composed from the
            // resolved player so it cannot disagree with what the database holds.
            let player_identity = player
                .gamertag
                .as_ref()
                .map(|gamertag| player.game.membership_key(gamertag).to_string());

            curia::info!("Connection authenticated", { "connection": connection_id.to_string(), "identity": authenticated_cn.to_string() });

            let spawner = self.session_spawner();
            tokio::spawn(async move {
                if let Err(e) = connection.keep_alive(true) {
                    curia::warn!("Keepalive failed {}: {}", connection_id, e);
                }
                let conn_arc = Arc::new(connection);

                spawner
                    .run(
                        SessionLink::Quic(conn_arc),
                        device,
                        player_identity,
                        fingerprint,
                    )
                    .await;
            });
        }
        Ok(())
    }
}
