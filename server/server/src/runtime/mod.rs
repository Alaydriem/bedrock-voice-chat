pub mod access_token;
pub mod assigned_name;
pub mod ca_store;
pub mod ca_cert;
pub mod enrollment;
pub mod node_key;
pub mod readiness;
pub mod secret_store;
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
pub use assigned_name::AssignedNameStore;
pub use ca_store::CaStore;
pub use node_key::NodeKeyStore;
pub use secret_store::{SecretName, SecretStore};
pub use readiness::ReadinessState;
pub use state::RuntimeState;

use anyhow::anyhow;
use common::curia;
use sea_orm::{ConnectOptions, Database, DatabaseConnection};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};

// 50 MB per file with ten archives bounds the directory near 500 MB. curia's
// defaults are desktop-tuned (40 KB, KeepOne) and are far too small here.
const MAX_LOG_FILE_SIZE: u64 = 50 * 1024 * 1024;
const LOG_ARCHIVES_KEPT: usize = 10;

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
    /// The held enrollment session, once one exists. Kept so the ACME provider and the
    /// challenge responder share the connection this server opened rather than each
    /// dialling their own — the relay pushes challenges down whichever one it holds.
    relay_enrollment: Arc<RwLock<Option<Arc<crate::services::RelayEnrollmentClient>>>>,
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
            relay_enrollment: Arc::new(RwLock::new(None)),
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
                curia::info!("Received {}, shutting down...", signal);
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
                curia::warn!("could not install SIGTERM handler: {}", e);
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

        curia::info!("Bedrock Voice Chat Server v{}", crate::VERSION);
        curia::info!(
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

        // Populated by the relay enrollment session's challenge responder, and read by
        // the unauthenticated route the relay fetches from the declared address.
        // Created here so it exists whether or not this server enrolls: the route then
        // answers 404 rather than being absent, which is a clearer thing for an
        // operator to see.
        let enrollment_nonce = crate::services::CurrentNonce::new_shared();

        // The CA keypair is generated exactly once per deployment, so its absence from both
        // the database and the certs directory means this boot is the deployment's first.
        let ca_minted = !CaStore::exists(db_conn.as_ref()).await?
            && !std::path::Path::new(&self.config.server.tls.certs_path)
                .join("ca.key")
                .exists();

        // Certificate material comes from exactly one of three places: an enrollment
        // token, manual paths, or an ACME block. Checked before any of them is acted
        // on, so a conflict between the other two is caught as well.
        if let Some(conflict) = self.config.server.tls_source_conflict() {
            return Err(anyhow!(conflict));
        }

        // Resolved whether or not peering is configured. Deferring this to the peering
        // branch below would mean an operator who enables peering later has already lost the
        // key, and every far-side `peer` block naming it would be dead.
        //
        // Resolved BEFORE the CA is signed, because enrollment needs this key and the CA
        // needs the name enrollment returns. Signing first would produce a CA whose SAN
        // set omits this server's own name; `SanKeySet` notices the drift on the NEXT
        // boot and re-signs, leaving the QUIC leaf wrong for a whole run with nothing in
        // the log saying so.
        let node_secret = node_key::NodeKeyStore::new(&self.config.server.tls.certs_path)
            .resolve(db_conn.as_ref())
            .await?;

        let assigned_name = self
            .resolve_assignment(db_conn.as_ref(), &node_secret)
            .await?;
        if let Some(name) = assigned_name.clone() {
            crate::runtime::enrollment::EnrollmentStep::apply(&mut self.config, name.clone());
            curia::info!("this server is reachable at its assigned name", { "url": format!("https://{name}") });
        }

        // Database-backed, materialised to disk. The TLS stacks take file paths and read them
        // once at ignite, so the bytes have to land somewhere readable — but the durable copy
        // lives in the database, which is what lets a container run without a persistent
        // volume.
        let (_ca_pem, _ca_key_pem) = self.generate_ca(db_conn.as_ref()).await?;

        // Resolved before any component clones the config. A configured value wins and is
        // mirrored into the database; otherwise the stored value is used, a pre-database
        // file is imported, or a fresh token is generated.
        let token_manager =
            access_token::AccessTokenManager::new(&self.config.server.tls.certs_path);
        self.config.server.minecraft.access_token = token_manager
            .resolve(db_conn.as_ref(), &self.config.server.minecraft.access_token)
            .await?;

        // ACME DNS-01. Issuance must complete before Rocket starts — the HTTPS
        // listener cannot exist without a certificate.
        let mut acme_service: Option<Arc<crate::services::acme::AcmeService>> = None;
        if let Some(acme_config) = self.config.server.tls.acme.clone() {
            let service = match acme_config.provider_kind()? {
                // The relay provider's only parameter is the live enrollment session,
                // which `Acme` cannot carry, so it is built here rather than from
                // configuration the way the others are.
                crate::config::AcmeProviderKind::BvcRelay => {
                    let name = assigned_name.clone().ok_or_else(|| {
                        anyhow!("the bvc-relay acme provider requires an assigned name")
                    })?;
                    let client = self
                        .relay_session(db_conn.as_ref(), &node_secret)
                        .await?
                        .ok_or_else(|| {
                            anyhow!("the bvc-relay acme provider requires a registry")
                        })?;
                    client.spawn_challenge_responder(
                        bvc_relay::node::NodeIdentity::from_secret_bytes(&node_secret)
                            .secret_key()
                            .clone(),
                        enrollment_nonce.clone(),
                    );

                    match self.config.server.enrollment.address() {
                        Some(address) => {
                            client.declare_address(address).await?;
                            curia::info!("published this server's address for the assigned name", { "address": address.to_string() });
                        }
                        // The certificate is valid and the name resolves to nothing, so
                        // every client fails to connect with a DNS error that names
                        // neither this server nor this setting. Said out loud because
                        // there is no other symptom to follow back here.
                        None => curia::warn!(
                            "no address declared, so the assigned name has no DNS record and nobody can reach this server by it; set server.enrollment.address to this server's public IP",
                            { "name": name.clone() }
                        ),
                    }

                    crate::services::acme::AcmeService::with_provider(
                        acme_config,
                        &self.config.server.tls.names,
                        &self.config.server.tls.certs_path,
                        db_conn.clone(),
                        crate::services::acme::provider::DnsProvider::from_relay(
                            client,
                            name,
                        ),
                    )?
                }
                _ => crate::services::acme::AcmeService::new(
                    acme_config,
                    &self.config.server.tls.names,
                    &self.config.server.tls.certs_path,
                    db_conn.clone(),
                )?,
            };
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
            curia::info!(
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
        let grants = Arc::new(
            crate::relay::GrantTable::from_config_and_db(
                &self.config.server.peers,
                db_conn.as_ref(),
            )
            .await?,
        );

        let mut peer_plane: Option<Arc<crate::relay::PeerPlane>> = None;

        if !self.config.server.peering_enabled() {
            curia::info!("peering is not configured; no peer socket bound");
        } else {
            let identity = bvc_relay::node::NodeIdentity::from_secret_bytes(&node_secret);

            let plane = crate::relay::PeerPlane::bind(
                &identity,
                Arc::clone(&grants),
                connection_registry.clone(),
                Arc::new(webhook_receiver.clone()),
                cache_manager.players().inner_arc(),
                self.config.server.peer_port,
            )
            .await?;

            // Logged at startup because an operator has no other way to read it,
            // and the other side's `peer` block needs exactly this string.
            curia::info!("peering enabled", { "node_id": plane.node_id().to_string(), "peers": grants.len() });

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
                        curia::info!("this server's peer link", { "peerlink": peerlink.to_string() })
                    }
                    Err(e) => curia::warn!(format!("could not mint this server's peer link ({e});                          `bvc-server relay peerlink` asks again on demand")),
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
            enrollment_nonce.clone(),
            peer_plane,
            #[cfg(feature = "bedrock")]
            transfer_target_cache.clone(),
        );

        self.state = RuntimeState::Running;

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
                curia::error!("Failed to start bedrock transfer relay: {}", e);
            }
            transfer_relay = Some(relay);

            for entry in &self.config.server.bedrock.servers {
                curia::info!(
                    "Advertising Bedrock server {} at {}:{} (addon transport: {:?})",
                    entry.name,
                    entry.host,
                    entry.port,
                    entry.addon_mode,
                );
            }
        } else {
            curia::info!(
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
                curia::error!("Failed to register with Meridian; heartbeat will retry", { "error": e.to_string() });
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
        curia::info!("WebSocket voice transport bound", { "bind": websocket_bind.to_string() });

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
                        Ok(_) => curia::info!("QUIC server stopped normally"),
                        Err(e) => curia::error!("QUIC server error: {}", e),
                    }
                    break;
                }
                result = &mut demux => {
                    match result {
                        Ok(_) => curia::info!("TLS demultiplexer stopped normally"),
                        Err(e) => curia::error!("TLS demultiplexer error: {}", e),
                    }
                    break;
                }
                result = &mut websocket => {
                    match result {
                        Ok(_) => curia::info!("WebSocket voice listener stopped normally"),
                        Err(e) => curia::error!("WebSocket voice listener error: {}", e),
                    }
                    break;
                }
                result = rocket.as_mut() => {
                    if relaunch_rocket {
                        relaunch_rocket = false;
                        curia::info!("Relaunching Rocket with renewed certificate");
                        rocket = Box::pin(rocket_manager.start());
                    } else {
                        match result {
                            Ok(_) => curia::info!("Rocket server stopped normally"),
                            Err(e) => curia::error!("Rocket server error: {}", e),
                        }
                        break;
                    }
                }
                Some(_) = acme_renewed_rx.recv() => {
                    curia::info!("ACME certificate renewed; bouncing the HTTP listener");
                    relaunch_rocket = true;
                    if let Err(e) = rocket_manager.stop().await {
                        curia::error!("Failed to stop Rocket for certificate reload: {}", e);
                    }
                }
                _ = shutdown_notify.notified() => {
                    curia::info!("Shutdown requested, shutting down...");
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
                    curia::error!("Failed to stop bedrock transfer relay: {}", e);
                }
            }
        }

        if let Err(e) = quic_manager.stop().await {
            curia::error!("Error stopping QUIC server: {}", e);
        }

        if let Err(e) = rocket_manager.stop().await {
            curia::error!("Error stopping Rocket server: {}", e);
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
        curia::info!(
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

    /// The level every sink is admitted at, and the global floor of the default
    /// directive string. Reproduces the EnvFilter strings this pipeline replaced,
    /// so an unset RUST_LOG filters exactly as before.
    pub fn default_directives(level: curia::Level) -> String {
        const QUIET: &str = "hyper=off,rustls=off,rocket::server=off,rocket_http::tls::listener=off,metrics_exporter_dogstatsd::forwarder=off";

        match level {
            curia::Level::Info => format!("info,{QUIET}"),
            curia::Level::Warn => format!("warn,{QUIET}"),
            curia::Level::Error => format!("error,{QUIET}"),
            curia::Level::Debug => "info,rocket_http::tls::listener=off".to_string(),
            curia::Level::Trace => "debug".to_string(),
        }
    }

    /// Install the logging pipeline: a coloured human console on stderr and a
    /// rotating JSON file, both unconditional.
    pub fn setup_logging(&mut self) -> Result<(), anyhow::Error> {
        use common::curia::{
            ConsoleSink, Dispatcher, FileOpenStrategy, FileSink, Filter, Logger, RotationStrategy,
            TimezoneStrategy, TracingBridge,
        };
        use tracing_subscriber::layer::SubscriberExt;
        use tracing_subscriber::util::SubscriberInitExt;

        use crate::logging::{HumanFormatter, JsonFormatter, LogContext, LogSinkType};

        // Windows consoles need ENABLE_VIRTUAL_TERMINAL_PROCESSING before an
        // escape sequence renders. Cross-platform; returns None elsewhere.
        let _ = anstyle_query::windows::enable_ansi_colors();

        let directives = std::env::var("RUST_LOG")
            .unwrap_or_else(|_| Self::default_directives(self.config.get_log_level()));
        let filter = Filter::from_directives(&directives);

        let context = LogContext::new_shared(self.config.server.meridian.as_ref());

        // Trace at the sink, so the dispatcher's filter is the only authority.
        let mut sinks = vec![LogSinkType::Console(ConsoleSink::new(
            curia::Level::Trace,
            HumanFormatter::new(HumanFormatter::detect_color()).formatter(),
        ))];

        // Logging must never take the server down with it. An unwritable log
        // directory degrades to console-only and says so once.
        let path = std::path::PathBuf::from(&self.config.log.path);
        match std::fs::create_dir_all(&path) {
            Err(e) => eprintln!("log directory unavailable, continuing without a file: {e}"),
            Ok(()) => match FileSink::with_rotation(
                path,
                "bvc-server".to_string(),
                curia::Level::Trace,
                JsonFormatter::new(context.clone()).formatter(),
                MAX_LOG_FILE_SIZE,
                RotationStrategy::KeepSome(LOG_ARCHIVES_KEPT),
                TimezoneStrategy::UseUtc,
                FileOpenStrategy::Append,
            ) {
                Ok(file) => sinks.push(LogSinkType::File(file)),
                Err(e) => eprintln!("log file unavailable, continuing without it: {e}"),
            },
        }

        let dispatcher = Dispatcher::new(sinks).with_filter(filter.clone());

        // A second runtime in this process finds the OnceLock claimed. curia
        // drops the rejected dispatcher, which closes the file and drains its
        // worker, so nothing is retained here on that path.
        if Logger::install(Box::new(dispatcher)).is_ok() {
            tracing_subscriber::registry()
                .with(TracingBridge::to_global().with_filter(filter))
                .init();
        }

        Ok(())
    }

    /// Ensure the QUIC server's CA cert and key exist and that the cert's SAN
    /// extension matches the configured `tls.names + tls.ips`. The key is
    /// generated exactly once per deployment; the cert is re-signed with the
    /// same key whenever the configured SAN set drifts. Returns
    /// `(cert_pem, key_pem)`.
    /// The assigned name, from the database if this server has enrolled before and
    /// from the relay if it has not.
    ///
    /// A stored name wins and the relay is never contacted, so an unreachable relay on
    /// a later boot is a non-event: the server starts on its own name with the
    /// certificate already on disk. Only a first enrollment needs the relay to answer.
    async fn resolve_assignment<C: sea_orm::ConnectionTrait>(
        &self,
        conn: &C,
        node_secret: &[u8; 32],
    ) -> Result<Option<String>, anyhow::Error> {
        // A build with no registry cannot renew an assigned name, publish a DNS record
        // for it, or reach the ACME provider `EnrollmentStep` switches to. A name stored
        // by an earlier build is therefore not read back either: applying it would
        // discard the configured ACME provider in favour of one that cannot run.
        if crate::config::Registry::peerlink().is_none() {
            return Ok(None);
        }

        if let Some(name) = assigned_name::AssignedNameStore::read(conn).await? {
            return Ok(Some(name));
        }

        let Some(token) = self.config.server.enrollment.token() else {
            return Ok(None);
        };

        let Some(client) = self.dial_registry(node_secret).await? else {
            return Ok(None);
        };

        let name = client.enroll(token).await?;
        assigned_name::AssignedNameStore::write(conn, &name).await?;

        curia::info!("this server enrolled with the relay registry", { "name": name.clone() });
        curia::warn!(
            "This server's assigned name is bound to its relay node key, which lives in \
             the database. Back the database up: a lost key means a new name, and the old \
             one is retired permanently rather than reissued."
        );

        Ok(Some(name))
    }

    /// The held enrollment session, dialling one if `resolve_assignment` did not.
    ///
    /// A server whose name came from the database never dialled, so the session it
    /// needs to publish a challenge does not exist yet.
    async fn relay_session<C: sea_orm::ConnectionTrait>(
        &self,
        _conn: &C,
        node_secret: &[u8; 32],
    ) -> Result<Option<Arc<crate::services::RelayEnrollmentClient>>, anyhow::Error> {
        if let Some(client) = self
            .relay_enrollment
            .read()
            .map_err(|_| anyhow!("relay enrollment lock poisoned"))?
            .clone()
        {
            return Ok(Some(client));
        }

        self.dial_registry(node_secret).await
    }

    /// The enrollment session, or `None` for a build with no registry baked in.
    ///
    /// A build without `BVC_REGISTRY_PEERLINK` has no registry to reach, so every
    /// feature that dials one is skipped rather than failed: the server runs on its
    /// configured name with the certificate it was given.
    async fn dial_registry(
        &self,
        node_secret: &[u8; 32],
    ) -> Result<Option<Arc<crate::services::RelayEnrollmentClient>>, anyhow::Error> {
        let Some(peerlink) = crate::config::Registry::peerlink() else {
            return Ok(None);
        };

        let addr = bvc_relay::node::PeerTicket::parse(&peerlink)
            .map_err(|e| anyhow!("the baked-in BVC_REGISTRY_PEERLINK is not a peer link: {e}"))?;

        let identity = bvc_relay::node::NodeIdentity::from_secret_bytes(node_secret);
        let client =
            crate::services::RelayEnrollmentClient::connect(&identity, addr, None).await?;

        self.relay_enrollment
            .write()
            .map_err(|_| anyhow!("relay enrollment lock poisoned"))?
            .replace(client.clone());

        Ok(Some(client))
    }

    async fn generate_ca<C: sea_orm::ConnectionTrait>(
        &self,
        conn: &C,
    ) -> Result<(String, String), anyhow::Error> {
        let mut san_names = self.config.server.tls.names.clone();
        san_names.append(&mut self.config.server.tls.ips.clone());
        ca_store::CaStore::ensure(conn, &self.config.server.tls.certs_path, &san_names).await
    }
}
