pub mod access_token;
pub mod ca_store;
pub mod ca_cert;
pub mod readiness;
pub mod position_updater;
pub mod state;
use crate::config::ApplicationConfig;
use crate::http::manager::RocketManager;
use crate::services::{
    AudioPlaybackService, BedrockEventService, CertificateService, EjectScheduler, MeridianService,
    PlayerIdentityService, PlayerRegistrarService,
};
use crate::stream::quic::{QuicServerManager, WebhookReceiver};
use common::traits::StreamTrait;
pub use ca_store::CaStore;
pub use readiness::ReadinessState;
pub use state::RuntimeState;

use anyhow::anyhow;
use faccess::PathExt;
use sea_orm::{ConnectOptions, Database, DatabaseConnection};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use tracing::info;
use tracing_appender::non_blocking::WorkerGuard;

/// Server runtime that manages the full BVC server stack.
/// This is the main entry point for both CLI and FFI usage.
pub struct ServerRuntime {
    config: ApplicationConfig,
    state: RuntimeState,
    shutdown_flag: Arc<AtomicBool>,
    shutdown_notify: Arc<tokio::sync::Notify>,
    /// Webhook receiver for sending position updates directly (populated after start)
    webhook_receiver: Arc<RwLock<Option<WebhookReceiver>>>,
    cache_manager: Arc<RwLock<Option<crate::stream::quic::CacheManager>>>,
    /// Published for the FFI so an embedded mod can drive chat without a socket.
    chat_service: Arc<RwLock<Option<Arc<crate::services::ChatService>>>>,
    /// Published for the FFI so an embedded mod can report facts about its own host.
    /// An external mod reports the same facts over HTTP; embedded has no socket to
    /// use, so it needs this or the population that matters most goes unmeasured.
    metrics: Arc<RwLock<Option<Arc<crate::services::MetricsService>>>>,
    /// Player registrar for handling player registration (populated after start)
    player_registrar: Arc<RwLock<Option<PlayerRegistrarService>>>,
    /// Player identity service for cross-platform name resolution (populated after start)
    identity_service: Arc<RwLock<Option<PlayerIdentityService>>>,
    audio_playback_service: Arc<RwLock<Option<Arc<AudioPlaybackService>>>>,
    db_conn: Arc<RwLock<Option<Arc<sea_orm::DatabaseConnection>>>>,
    _logger_guard: Option<WorkerGuard>,
}

impl ServerRuntime {
    /// Create a new runtime with ApplicationConfig
    pub fn new(config: ApplicationConfig) -> Result<Self, anyhow::Error> {
        Ok(Self {
            config,
            state: RuntimeState::Stopped,
            shutdown_flag: Arc::new(AtomicBool::new(false)),
            shutdown_notify: Arc::new(tokio::sync::Notify::new()),
            webhook_receiver: Arc::new(RwLock::new(None)),
            cache_manager: Arc::new(RwLock::new(None)),
            chat_service: Arc::new(RwLock::new(None)),
            metrics: Arc::new(RwLock::new(None)),
            player_registrar: Arc::new(RwLock::new(None)),
            identity_service: Arc::new(RwLock::new(None)),
            audio_playback_service: Arc::new(RwLock::new(None)),
            db_conn: Arc::new(RwLock::new(None)),
            _logger_guard: None,
        })
    }

    /// Create a new runtime from JSON config string
    pub fn from_json(json: &str) -> Result<Self, anyhow::Error> {
        let config: ApplicationConfig = serde_json::from_str(json)
            .map_err(|e| anyhow!("Failed to parse config JSON: {}", e))?;
        Self::new(config)
    }

    /// Get a reference to the config
    pub fn config(&self) -> &ApplicationConfig {
        &self.config
    }

    /// Get the current runtime state
    pub fn state(&self) -> RuntimeState {
        self.state
    }

    /// Check if shutdown has been requested
    pub fn is_shutdown_requested(&self) -> bool {
        self.shutdown_flag.load(Ordering::SeqCst)
    }

    /// Get a clone of the shutdown flag for external monitoring
    pub fn shutdown_flag(&self) -> Arc<AtomicBool> {
        self.shutdown_flag.clone()
    }

    pub fn shutdown_notify(&self) -> Arc<tokio::sync::Notify> {
        self.shutdown_notify.clone()
    }

    /// Start the server with shutdown signal handling.
    /// Blocks until the server shuts down.
    pub async fn start(&mut self) -> Result<(), anyhow::Error> {
        let shutdown_flag = self.shutdown_flag();
        let shutdown_notify = self.shutdown_notify();
        tokio::spawn(async move {
            if let Some(signal) = Self::await_shutdown_signal().await {
                tracing::info!("Received {}, shutting down...", signal);
                shutdown_flag.store(true, Ordering::SeqCst);
                shutdown_notify.notify_one();
            }
        });

        self.start_async().await
    }

    // Container runtimes and init systems stop a process with SIGTERM, never SIGINT.
    // Without the SIGTERM arm the default disposition kills the process before the
    // shutdown path runs, so a routine `docker stop` emits no Server::Stopped and is
    // indistinguishable downstream from a crash.
    #[cfg(unix)]
    async fn await_shutdown_signal() -> Option<&'static str> {
        use tokio::signal::unix::{SignalKind, signal};

        let mut terminate = match signal(SignalKind::terminate()) {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!("could not install SIGTERM handler: {}", e);
                return tokio::signal::ctrl_c().await.ok().map(|_| "CTRL+C");
            }
        };

        tokio::select! {
            result = tokio::signal::ctrl_c() => result.ok().map(|_| "CTRL+C"),
            _ = terminate.recv() => Some("SIGTERM"),
        }
    }

    #[cfg(not(unix))]
    async fn await_shutdown_signal() -> Option<&'static str> {
        tokio::signal::ctrl_c().await.ok().map(|_| "CTRL+C")
    }

    /// Initialize and start the server (async)
    pub async fn start_async(&mut self) -> Result<(), anyhow::Error> {
        if self.state != RuntimeState::Stopped {
            return Err(anyhow!("Server is already running or starting"));
        }

        self.state = RuntimeState::Starting;

        // Setup logging
        self.setup_logging()?;

        info!("Bedrock Voice Chat Server v{}", crate::VERSION);
        info!(
            "Protocol Version: {}",
            common::consts::version::PROTOCOL_VERSION
        );

        // The database connection and its schema are established before anything that needs
        // them, which now includes the CA. Migrations otherwise run inside Rocket's ignite
        // fairing, long after this point, so a table read here would not yet exist.
        let db_conn = self.create_database_connection().await?;
        let db_conn = Arc::new(db_conn);
        {
            use migration::MigratorTrait;
            migration::Migrator::up(db_conn.as_ref(), None)
                .await
                .map_err(|e| anyhow!("running migrations: {}", e))?;
        }

        // The CA keypair is generated exactly once per deployment, so its absence from both
        // the database and the certs directory means this boot is the deployment's first.
        let ca_minted = !CaStore::exists(db_conn.as_ref()).await?
            && !std::path::Path::new(&self.config.server.tls.certs_path)
                .join("ca.key")
                .exists();

        // Database-backed, materialised to disk. The TLS stacks take file paths and read them
        // once at ignite, so the bytes have to land somewhere readable — but the durable copy
        // lives in the database, which is what lets a container run without a persistent
        // volume.
        let (_ca_pem, _ca_key_pem) = self.generate_ca(db_conn.as_ref()).await?;

        // Resolve the Minecraft access token before any component clones the
        // config. Env and config values win; otherwise the persisted token is
        // reused or a fresh one is generated once and logged.
        let token_manager =
            access_token::AccessTokenManager::new(&self.config.server.tls.certs_path);
        self.config.server.minecraft.access_token = token_manager
            .resolve(&self.config.server.minecraft.access_token)?;

        // ACME DNS-01: mutually exclusive with manual cert paths. Issuance
        // must complete before Rocket starts — the HTTPS listener cannot
        // exist without a certificate.
        let mut acme_service: Option<Arc<crate::services::acme::AcmeService>> = None;
        if let Some(acme_config) = self.config.server.tls.acme.clone() {
            if !self.config.server.tls.certificate.is_empty()
                || !self.config.server.tls.key.is_empty()
            {
                return Err(anyhow!(
                    "tls.acme and tls.certificate/tls.key are mutually exclusive; remove one"
                ));
            }
            let service = crate::services::acme::AcmeService::new(
                acme_config,
                &self.config.server.tls.names,
                &self.config.server.tls.certs_path,
            )?;
            let paths = service.ensure_certificate().await?;
            self.config.server.tls.certificate = paths.certificate;
            self.config.server.tls.key = paths.key;
            acme_service = Some(Arc::new(service));
        }

        // Create certificate manager (caches root CA)
        let cert_manager = CertificateService::new_shared(&self.config.server.tls.certs_path)?;
        let cert_service = Arc::new(CertificateService::new(&self.config.server.tls.certs_path)?);

        // One instance for the whole process. The HTTP guard, the QUIC handshake and the
        // WebSocket upgrade all consult it, and a second instance would carry its own cache —
        // so a revocation written through one would be invisible to the others.
        let certificate_revocations =
            crate::services::CertificateRevocationService::new_shared();

        // Authorizes the certificate presented at a QUIC or WebSocket handshake. Shares the
        // revocation list above, so a ban written over HTTP is seen by both transports.
        let session_authorization =
            crate::services::SessionAuthorizationService::new_shared(
                certificate_revocations.clone(),
            );

        // Create player registrar for shared player registration logic
        let player_registrar = PlayerRegistrarService::new(db_conn.clone(), cert_manager);

        // Create player identity service for cross-platform name resolution
        let identity_service = PlayerIdentityService::new(db_conn.clone());

        // Store player_registrar for FFI access
        {
            let mut pr = self
                .player_registrar
                .write()
                .map_err(|_| anyhow!("player_registrar lock poisoned"))?;
            *pr = Some(player_registrar.clone());
        }

        // Store identity_service for FFI access
        {
            let mut is = self
                .identity_service
                .write()
                .map_err(|_| anyhow!("identity_service lock poisoned"))?;
            *is = Some(identity_service.clone());
        }

        // QUIC server manager
        let mut quic_manager = QuicServerManager::new(
            self.config.clone(),
            session_authorization.clone(),
            db_conn.clone(),
        );
        let readiness_state = readiness::ReadinessState::new_shared();
        quic_manager.set_readiness(readiness_state.clone());
        let webhook_receiver = quic_manager.get_webhook_receiver().clone();
        let cache_manager = quic_manager.get_cache_manager();
        let connection_registry = quic_manager.get_connection_registry();

        // relay is deliberately absent: RelayFeature holds only tuning intervals and
        // has no enable flag, so there is no boolean to report.
        let mut features_enabled: Vec<String> = Vec::new();
        if self.config.server.features.openapi_docs {
            features_enabled.push("openapi_docs".to_string());
        }
        if self.config.server.features.telemetry {
            features_enabled.push("telemetry".to_string());
        }
        if self.config.server.meridian.is_some() {
            features_enabled.push("meridian".to_string());
        }
        if self.config.server.tls.acme.is_some() {
            features_enabled.push("acme".to_string());
        }
        if self.config.server.bedrock.enabled {
            features_enabled.push("bedrock".to_string());
        }

        // Gauges are pushed into the service by ConnectionRegistry on change. The
        // heartbeat task below owns the only other periodic work; the channel reaper
        // runs as an arm of the main event loop (structured cancellation — no detached
        // task). The boot ping is emitted inside new_shared. posthog_handle is awaited
        // on shutdown for a final flush.
        let (metrics, posthog_handle) = crate::services::MetricsService::new_shared(
            self.config.server.features.telemetry,
            &self.config.server.tls.certs_path,
            &self.config.server.tls.certificate,
            features_enabled,
            ca_minted,
            self.config.voice.recording.enabled,
            Some(cache_manager.player_state().clone()),
        );
        connection_registry.set_metrics(metrics.clone());

        let capacity = crate::stream::quic::connection::CapacityPolicy::new(
            self.config.voice.limits.connections,
            std::time::Duration::from_secs(self.config.voice.limits.reconnect_grace),
        );
        connection_registry.set_capacity(capacity);
        metrics.set_voice_capacity_limit(self.config.voice.limits.connections as i64);

        if self.config.voice.limits.connections > 0 {
            tracing::info!(
                "Voice capacity limited to {} concurrent sessions, {}s reconnect grace",
                self.config.voice.limits.connections,
                self.config.voice.limits.reconnect_grace
            );
        }

        if let Ok(mut slot) = self.metrics.write() {
            *slot = Some(metrics.clone());
        }

        let heartbeat_shutdown = tokio_util::sync::CancellationToken::new();
        let heartbeat_handle = metrics.spawn_heartbeat(heartbeat_shutdown.clone());

        let relay_watch_shutdown = tokio_util::sync::CancellationToken::new();
        let mut relay_watch_handle: Option<tokio::task::JoinHandle<()>> = None;

        // Shared stream-token cache: single-use tokens minted into the SAME cache
        // the public `/api/audio/stream` route validates against.
        let audio_stream_token_cache = crate::services::AudioStreamTokenCache::new();

        // Cross-server peering. Declared, never discovered: every peer is named in
        // `config.hcl`, and a config error here is fatal rather than a silently
        // unauthorized peer later.
        let grants = Arc::new(crate::relay::GrantTable::from_config(
            &self.config.server.peers,
        )?);

        let mut peer_plane: Option<Arc<crate::relay::PeerPlane>> = None;

        if grants.is_empty() {
            tracing::info!("peering is not configured; no peer socket bound");
        } else {
            let identity =
                bvc_relay::node::NodeIdentity::load_or_create(&self.config.server.tls.certs_path)?;

            let relay_url = match &self.config.server.peer_relay_url {
                Some(url) => match url.parse() {
                    Ok(url) => Some(url),
                    Err(_) => None
                },
                None => None
            };

            let plane = crate::relay::PeerPlane::bind(
                &identity,
                Arc::clone(&grants),
                connection_registry.clone(),
                Arc::new(webhook_receiver.clone()),
                cache_manager.players().inner_arc(),
                relay_url,
                self.config.server.peer_port,
            )
            .await?;

            // Logged at startup because an operator has no other way to read it,
            // and the other side's `peer` block needs exactly this string.
            tracing::info!(
                node_id = %plane.node_id(),
                peers = grants.len(),
                "peering enabled"
            );

            // Minted off the boot path rather than on it: `ticket()` waits up to two
            // seconds for iroh to report this endpoint's addresses, and the listeners
            // behind this line should not wait with it.
            //
            // Logged because the peer link is what the far side actually needs, and
            // `node_id` above is not it. An operator reading only the startup log had
            // to discover `bvc-server relay peerlink` to get the one string the other
            // side's config requires.
            let announced = plane.clone();
            tokio::spawn(async move {
                match announced.endpoint().ticket().await {
                    Ok(peerlink) => {
                        tracing::info!(peerlink = %peerlink, "this server's peer link")
                    }
                    Err(e) => tracing::warn!(
                        "could not mint this server's peer link ({e});                          `bvc-server relay peerlink` asks again on demand"
                    ),
                }
            });

            plane.spawn_accept_loop();
            connection_registry.set_peer_plane(plane.clone());
            peer_plane = Some(plane);

            // Gated on peering rather than unconditional: on a server with no
            // peer block these lines report a value nothing consumes, and the
            // `relay worlds` command answers the same question on demand.
            relay_watch_handle = Some(crate::relay::RelayWorldWatch::spawn(
                cache_manager.clone(),
                Arc::clone(&grants),
                relay_watch_shutdown.clone(),
            ));
        }

        // Store webhook_receiver for FFI position updates
        {
            let mut wr = self
                .webhook_receiver
                .write()
                .map_err(|_| anyhow!("webhook_receiver lock poisoned"))?;
            *wr = Some(webhook_receiver.clone());
        }

        // Store cache_manager for FFI control-plane routing.
        {
            let mut cm = self
                .cache_manager
                .write()
                .map_err(|_| anyhow!("cache_manager lock poisoned"))?;
            *cm = Some(cache_manager.clone());
        }

        // Create audio playback service. When the relay client is wired, the peer
        // manager doubles as the cross-server jukebox discovery handle so a local
        // miss can fetch the `.opus` from a peer; otherwise discovery is absent and
        // a miss is a hard error.
        let playback_cancel_token = tokio_util::sync::CancellationToken::new();
        let audio_playback_service = Arc::new(AudioPlaybackService::new(
            webhook_receiver.clone(),
            self.config.audio.file_path.clone(),
            playback_cancel_token.clone(),
            self.config.audio.max_concurrent_per_uuid,
        ));

        // The audio path resolves a server-injected speaker from this registry rather than
        // from the position cache, whose TTL is a presence lifetime and would lapse part-way
        // through a track. Shared through a `OnceLock`, so the clone the QUIC path already
        // holds sees it too.
        cache_manager.set_injected_speakers(&audio_playback_service);

        // Store audio_playback_service and db_conn for FFI access
        {
            let mut aps = self
                .audio_playback_service
                .write()
                .map_err(|_| anyhow!("audio_playback_service lock poisoned"))?;
            *aps = Some(audio_playback_service.clone());
        }
        {
            let mut dc = self
                .db_conn
                .write()
                .map_err(|_| anyhow!("db_conn lock poisoned"))?;
            *dc = Some(db_conn.clone());
        }

        // Bedrock proxy event ingress: sniffs in-game events from Proxy/Realms Connect
        // clients and dispatches them through the same services BDS HTTP routes call into.
        let bedrock_event_service = BedrockEventService::new_shared(
            audio_playback_service.clone(),
            webhook_receiver.clone(),
            db_conn.clone(),
            self.config
                .server
                .bedrock
                .proxy_event_freshness_threshold_secs,
        );
        quic_manager.set_bedrock_event_service(bedrock_event_service.clone());
        quic_manager.set_control_webhook_receiver(webhook_receiver.clone());

        // Net-mode chat hub. Its dependencies arrive by setter rather than constructor so the
        // service's own tests can exercise routing without a database or a QUIC registry.
        let chat_service =
            crate::services::ChatService::new_shared(self.config.server.features.chat);
        chat_service.set_db(db_conn.clone());
        chat_service.set_identities(std::sync::Arc::new(identity_service.clone()));
        chat_service.set_players(cache_manager.players().inner_arc());
        if let Some(registry) = cache_manager.get_connection_registry() {
            chat_service.add_sink(crate::services::QuicChatSink::new_shared(
                registry,
                cache_manager.players().inner_arc(),
            ));
        }

        quic_manager.set_chat_service(chat_service.clone());

        if let Ok(mut slot) = self.chat_service.write() {
            *slot = Some(chat_service.clone());
        }

        let eject_scheduler =
            EjectScheduler::new_shared(bedrock_event_service.clone(), webhook_receiver.clone());
        audio_playback_service.set_eject_scheduler(eject_scheduler);

        #[cfg(feature = "bedrock")]
        let transfer_target_cache = crate::services::bedrock::TransferTargetCache::new(
            self.config.server.bedrock.transfer_cache_ttl_secs,
        );

        // The API listener moves to loopback and the TLS demultiplexer takes the public
        // port, so one hostname and one certificate serve the API, the browser feeds and
        // the WebSocket voice transport. Resolved once here rather than per launch: an
        // ACME renewal relaunches Rocket, and a fresh port each time would leave the
        // demultiplexer relaying to an address nothing is listening on.
        // Shared rather than a plain address: `LoopbackPort` picks by binding port
        // zero and releasing, so the number can be taken before Rocket binds it.
        // Rocket re-picks in that case, and the demultiplexer reads the same cell
        // so it relays to wherever the listener actually landed.
        let api_bind = crate::demux::ApiBind::reserve()?;

        // Create Rocket manager
        let rocket_manager = RocketManager::new(
            self.config.clone(),
            api_bind.clone(),
            webhook_receiver,
            cache_manager,
            player_registrar,
            identity_service,
            audio_playback_service,
            bedrock_event_service,
            chat_service,
            cert_service,
            certificate_revocations.clone(),
            Some(audio_stream_token_cache),
            metrics.clone(),
            readiness_state.clone(),
            peer_plane,
            #[cfg(feature = "bedrock")]
            transfer_target_cache.clone(),
        );

        self.state = RuntimeState::Running;

        #[cfg(feature = "bedrock")]
        #[cfg(feature = "bedrock")]
        let mut transfer_relay = None;

        #[cfg(feature = "bedrock")]
        if self.config.server.bedrock.enabled {
            use common::traits::StreamTrait;

            let mut relay = crate::services::bedrock::TransferRelayService::new(
                self.config.server.bedrock.transfer_port,
                transfer_target_cache.clone(),
            );
            if let Err(e) = relay.start().await {
                tracing::error!("Failed to start bedrock transfer relay: {}", e);
            }
            transfer_relay = Some(relay);

            for entry in &self.config.server.bedrock.servers {
                tracing::info!(
                    "Advertising Bedrock server {} at {}:{} (addon transport: {:?})",
                    entry.name,
                    entry.host,
                    entry.port,
                    entry.addon_mode,
                );
            }
        } else {
            tracing::info!(
                "Bedrock services disabled (server.bedrock.enabled = false); DNS and transfer relay not started"
            );
        }

        // Register with Meridian if configured. Cancelled after the main event
        // loop exits, so the heartbeat is torn down with everything else rather
        // than detached.
        let meridian_shutdown = tokio_util::sync::CancellationToken::new();
        let mut meridian_heartbeat: Option<tokio::task::JoinHandle<()>> = None;

        if let Some(meridian_config) = &self.config.server.meridian {
            let hostname = meridian_config
                .host
                .clone()
                .or_else(|| {
                    self.config
                        .server
                        .tls
                        .names
                        .iter()
                        .find(|name| name.parse::<std::net::IpAddr>().is_err())
                        .cloned()
                })
                .unwrap_or_else(|| "localhost".to_string());

            let service = MeridianService::new(
                meridian_config.clone(),
                meridian_config.backend.clone(),
                self.config.server.port,
                self.config.server.quic_port,
                hostname,
            );

            // Attempt once inline so a misconfiguration is visible at startup, then
            // keep refreshing. A one-shot registration leaves this customer
            // unroutable if Meridian restarts or this record's lease lapses.
            if let Err(e) = service.register().await {
                tracing::error!(
                    error = %e,
                    "Failed to register with Meridian; heartbeat will retry"
                );
            }

            meridian_heartbeat =
                Some(std::sync::Arc::new(service).spawn_heartbeat(meridian_shutdown.clone()));
        }

        let _shutdown_flag = self.shutdown_flag.clone();
        let shutdown_notify = self.shutdown_notify.clone();

        // Renewal plumbing: the ACME task signals here after re-issuing; the
        // loop below gracefully bounces only the Rocket listener. With ACME
        // off, the sender drops immediately and the arm never fires.
        let acme_cancel = tokio_util::sync::CancellationToken::new();
        let (acme_renewed_tx, mut acme_renewed_rx) = tokio::sync::mpsc::channel::<()>(1);
        let mut acme_renewal_task: Option<tokio::task::JoinHandle<()>> = None;
        if let Some(service) = &acme_service {
            acme_renewal_task =
                Some(service.clone().spawn_renewal(acme_cancel.clone(), acme_renewed_tx));
        } else {
            drop(acme_renewed_tx);
        }

        // The public TLS port. A peer of QUIC and Rocket rather than an optional extra:
        // nothing reaches the API or the voice transport if this cannot bind, so it is an
        // arm of the loop below and its failure stops the server rather than being logged
        // into a server that then serves nobody.
        let public_listen_ip: std::net::IpAddr = self
            .config
            .server
            .unbracketed_listen()
            .parse()
            .map_err(|e| {
                anyhow!(
                    "server.listen = \"{}\" is not an IP address: {e}",
                    self.config.server.listen
                )
            })?;
        let public_port = u16::try_from(self.config.server.port).map_err(|_| {
            anyhow!(
                "server.port = {} is outside the range of a TCP port",
                self.config.server.port
            )
        })?;
        // The WebSocket voice transport. Bound before the demultiplexer so the address is
        // known when it is handed over, and so its readiness gate has something to wait
        // for rather than a port nothing will ever answer on.
        let (mut websocket_listener, websocket_bind) = crate::stream::session::WebSocketListener::bind(
            &self.config.server.tls.certificate,
            &self.config.server.tls.key,
            &format!("{}/ca.crt", self.config.server.tls.certs_path),
            quic_manager.session_spawner(),
            session_authorization.clone(),
            db_conn.clone(),
        )
        .await?;
        websocket_listener.set_metrics(metrics.clone());
        tracing::info!(bind = %websocket_bind, "WebSocket voice transport bound");

        let demux = crate::demux::AlpnDemux::new(
            std::net::SocketAddr::new(public_listen_ip, public_port),
            api_bind,
            Some(websocket_bind),
        );

        // Main event loop: run QUIC + Rocket + the demultiplexer until one stops or
        // shutdown is requested, with the low-cadence channel reaper as a structured arm.
        // On exit the pinned futures drop (structured cancellation) — no detached task, no
        // separate shutdown wiring for the periodic work.
        // Note: CTRL+C handling is done by the host process (Java/CLI), not here.
        {
        let quic = quic_manager.start();
        tokio::pin!(quic);
        let demux = demux.start();
        tokio::pin!(demux);
        let websocket = websocket_listener.start();
        tokio::pin!(websocket);
        let mut rocket = Box::pin(rocket_manager.start());
        let mut relaunch_rocket = false;
        let mut reap_interval = tokio::time::interval(std::time::Duration::from_secs(60));
        loop {
            tokio::select! {
                result = &mut quic => {
                    match result {
                        Ok(_) => tracing::info!("QUIC server stopped normally"),
                        Err(e) => tracing::error!("QUIC server error: {}", e),
                    }
                    break;
                }
                result = &mut demux => {
                    match result {
                        Ok(_) => tracing::info!("TLS demultiplexer stopped normally"),
                        Err(e) => tracing::error!("TLS demultiplexer error: {}", e),
                    }
                    break;
                }
                result = &mut websocket => {
                    match result {
                        Ok(_) => tracing::info!("WebSocket voice listener stopped normally"),
                        Err(e) => tracing::error!("WebSocket voice listener error: {}", e),
                    }
                    break;
                }
                result = rocket.as_mut() => {
                    if relaunch_rocket {
                        relaunch_rocket = false;
                        tracing::info!("Relaunching Rocket with renewed certificate");
                        rocket = Box::pin(rocket_manager.start());
                    } else {
                        match result {
                            Ok(_) => tracing::info!("Rocket server stopped normally"),
                            Err(e) => tracing::error!("Rocket server error: {}", e),
                        }
                        break;
                    }
                }
                Some(_) = acme_renewed_rx.recv() => {
                    tracing::info!("ACME certificate renewed; bouncing the HTTP listener");
                    relaunch_rocket = true;
                    if let Err(e) = rocket_manager.stop().await {
                        tracing::error!("Failed to stop Rocket for certificate reload: {}", e);
                    }
                }
                _ = shutdown_notify.notified() => {
                    tracing::info!("Shutdown requested, shutting down...");
                    break;
                }
                _ = reap_interval.tick() => {
                    connection_registry.reap_stale_channels();
                }
            }
        }
        }

        acme_cancel.cancel();
        if let Some(handle) = acme_renewal_task {
            let _ = tokio::time::timeout(std::time::Duration::from_secs(2), handle).await;
        }

        // Stop refreshing the Meridian record. The lease then lapses on Meridian's
        // side, which is how a departing backend is removed without an explicit
        // delete.
        meridian_shutdown.cancel();
        if let Some(handle) = meridian_heartbeat {
            let _ = tokio::time::timeout(std::time::Duration::from_secs(2), handle).await;
        }

        heartbeat_shutdown.cancel();
        let _ = tokio::time::timeout(std::time::Duration::from_secs(2), heartbeat_handle).await;

        relay_watch_shutdown.cancel();
        if let Some(handle) = relay_watch_handle {
            let _ = tokio::time::timeout(std::time::Duration::from_secs(2), handle).await;
        }

        metrics.record_stopped();

        // Final PostHog flush: signal the drain and await it briefly so buffered fleet
        // events are sent before teardown.
        metrics.begin_posthog_drain();
        if let Some(handle) = posthog_handle {
            // Must exceed the PostHog client's own 10s per-request timeout, or a
            // single slow request consumes the whole budget and the drained buffer —
            // Server::Stopped included — is abandoned.
            let _ = tokio::time::timeout(std::time::Duration::from_secs(15), handle).await;
        }

        // Always stop QUIC regardless of which branch exited
        self.state = RuntimeState::ShuttingDown;

        #[cfg(feature = "bedrock")]
        {
            use common::traits::StreamTrait;
            if let Some(ref mut relay) = transfer_relay {
                if let Err(e) = relay.stop().await {
                    tracing::error!("Failed to stop bedrock transfer relay: {}", e);
                }
            }
        }

        if let Err(e) = quic_manager.stop().await {
            tracing::error!("Error stopping QUIC server: {}", e);
        }

        if let Err(e) = rocket_manager.stop().await {
            tracing::error!("Error stopping Rocket server: {}", e);
        }

        self.state = RuntimeState::Stopped;
        Ok(())
    }

    /// Signal the server to stop gracefully
    pub fn request_shutdown(&self) {
        self.shutdown_flag.store(true, Ordering::SeqCst);
        self.shutdown_notify.notify_one();
    }

    /// Get a clone of the webhook receiver Arc for external use
    pub fn get_webhook_receiver(&self) -> Arc<RwLock<Option<WebhookReceiver>>> {
        self.webhook_receiver.clone()
    }

    /// Get a clone of the cache manager Arc for external use (FFI control plane)
    pub fn get_cache_manager(&self) -> Arc<RwLock<Option<crate::stream::quic::CacheManager>>> {
        self.cache_manager.clone()
    }

    /// The chat hub, for the FFI.
    ///
    /// An embedded mod shares this process, so it drives chat through function calls rather
    /// than dialling a socket back into its own address space.
    pub fn get_chat_service(&self) -> Arc<RwLock<Option<Arc<crate::services::ChatService>>>> {
        self.chat_service.clone()
    }

    /// Get a clone of the metrics Arc for external use (FFI)
    pub fn get_metrics(&self) -> Arc<RwLock<Option<Arc<crate::services::MetricsService>>>> {
        self.metrics.clone()
    }

    /// Get a clone of the player registrar Arc for external use (FFI)
    pub fn get_player_registrar(&self) -> Arc<RwLock<Option<PlayerRegistrarService>>> {
        self.player_registrar.clone()
    }

    /// Get a clone of the identity service Arc for external use (FFI)
    pub fn get_identity_service(&self) -> Arc<RwLock<Option<PlayerIdentityService>>> {
        self.identity_service.clone()
    }

    pub fn get_audio_playback_service(&self) -> Arc<RwLock<Option<Arc<AudioPlaybackService>>>> {
        self.audio_playback_service.clone()
    }

    pub fn get_db_conn(&self) -> Arc<RwLock<Option<Arc<sea_orm::DatabaseConnection>>>> {
        self.db_conn.clone()
    }

    /// Update player positions directly (bypasses HTTP).
    /// Used by FFI to send position updates without HTTP overhead.
    ///
    /// # Arguments
    /// * `players` - Vector of player position data
    ///
    /// # Returns
    /// * Ok(()) on success
    /// * Err if server not started or webhook_receiver not available
    pub async fn update_positions(
        &self,
        players: Vec<common::PlayerEnum>,
    ) -> Result<(), anyhow::Error> {
        let wr_guard = self
            .webhook_receiver
            .read()
            .map_err(|_| anyhow!("Failed to acquire webhook_receiver lock"))?;

        let webhook_receiver = wr_guard
            .as_ref()
            .ok_or_else(|| anyhow!("Server not started - webhook_receiver not available"))?;

        position_updater::PositionUpdater::broadcast_positions(players, webhook_receiver).await;
        Ok(())
    }

    /// Create a standalone database connection.
    /// This is used by the PlayerRegistrarService and can be shared between components.
    async fn create_database_connection(&self) -> Result<DatabaseConnection, anyhow::Error> {
        self.config.database.validate()?;
        tracing::info!(
            "Creating standalone database connection: {}",
            self.config.database.get_redacted_dsn()
        );

        let mut options = ConnectOptions::new(self.config.database.get_dsn());
        options
            .max_connections(100)
            .min_connections(1)
            .connect_timeout(std::time::Duration::from_secs(3))
            .idle_timeout(std::time::Duration::from_secs(60))
            .sqlx_logging(false);

        let conn = Database::connect(options).await?;
        Ok(conn)
    }

    /// Setup the tracing/logging subsystem
    fn setup_logging(&mut self) -> Result<(), anyhow::Error> {
        use tracing_appender::non_blocking::NonBlocking;
        use tracing_subscriber::fmt::SubscriberBuilder;

        let out = &self.config.log.out;
        let subscriber: SubscriberBuilder = tracing_subscriber::fmt();
        let non_blocking: NonBlocking;
        let guard: WorkerGuard;

        match out.to_lowercase().as_str() {
            "stdout" | "callback" => {
                (non_blocking, guard) = tracing_appender::non_blocking(std::io::stdout());
            }
            _ => {
                let path = Path::new(out);
                if !path.exists() || !path.writable() {
                    return Err(anyhow!("{} doesn't exist or is not writable", out));
                }
                let file_appender = tracing_appender::rolling::daily(out, "bvc-server.log");
                (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
            }
        }

        let env_filter = match self.config.get_tracing_log_level() {
            tracing::Level::INFO => {
                "info,hyper=off,rustls=off,rocket::server=off,rocket_http::tls::listener=off,metrics_exporter_dogstatsd::forwarder=off"
            }
            tracing::Level::DEBUG => "info,rocket_http::tls::listener=off",
            tracing::Level::TRACE => "debug",
            tracing::Level::ERROR => {
                "error,hyper=off,rustls=off,rocket::server=off,rocket_http::tls::listener=off,metrics_exporter_dogstatsd::forwarder=off"
            }
            tracing::Level::WARN => {
                "warn,hyper=off,rustls=off,rocket::server=off,rocket_http::tls::listener=off,metrics_exporter_dogstatsd::forwarder=off"
            }
        };

        let installed = subscriber
            .with_writer(non_blocking)
            .with_max_level(self.config.get_tracing_log_level())
            .with_level(true)
            .with_line_number(&self.config.log.level == "trace")
            .with_file(&self.config.log.level == "trace")
            .with_env_filter(env_filter)
            .with_ansi(true)
            .compact()
            .try_init()
            .is_ok();

        if installed {
            self._logger_guard = Some(guard);
        } else {
            // Another runtime in this process already owns the global subscriber,
            // so drop our worker guard rather than retaining a writer that is
            // never wired up.
            self._logger_guard = None;
        }

        Ok(())
    }

    /// Ensure the QUIC server's CA cert and key exist and that the cert's SAN
    /// extension matches the configured `tls.names + tls.ips`. The key is
    /// generated exactly once per deployment; the cert is re-signed with the
    /// same key whenever the configured SAN set drifts. Returns
    /// `(cert_pem, key_pem)`.
    async fn generate_ca<C: sea_orm::ConnectionTrait>(
        &self,
        conn: &C,
    ) -> Result<(String, String), anyhow::Error> {
        let mut san_names = self.config.server.tls.names.clone();
        san_names.append(&mut self.config.server.tls.ips.clone());
        ca_store::CaStore::ensure(conn, &self.config.server.tls.certs_path, &san_names).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ApplicationConfig;

    #[test]
    fn logging_init_is_idempotent_within_a_process() {
        let mut first = ServerRuntime::new(ApplicationConfig::default()).unwrap();
        let mut second = ServerRuntime::new(ApplicationConfig::default()).unwrap();

        first.setup_logging().unwrap();
        second.setup_logging().unwrap();
    }
}
