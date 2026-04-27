pub mod ca_cert;
pub mod position_updater;
pub mod state;
pub use state::RuntimeState;
use crate::config::ApplicationConfig;
use crate::http::manager::RocketManager;
use crate::services::{AudioPlaybackService, CertificateService, MeridianService, PlayerIdentityService, PlayerRegistrarService};
use crate::stream::quic::{QuicServerManager, WebhookReceiver};

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

    /// Start the server with CTRL+C signal handling.
    /// Blocks until the server shuts down.
    pub async fn start(&mut self) -> Result<(), anyhow::Error> {
        let shutdown_flag = self.shutdown_flag();
        let shutdown_notify = self.shutdown_notify();
        tokio::spawn(async move {
            if let Ok(()) = tokio::signal::ctrl_c().await {
                tracing::info!("Received CTRL+C, shutting down...");
                shutdown_flag.store(true, Ordering::SeqCst);
                shutdown_notify.notify_one();
            }
        });

        self.start_async().await
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
        info!("Protocol Version: {}", common::consts::version::PROTOCOL_VERSION);

        // Generate CA certificates
        self.generate_ca().await?;

        // Create standalone database connection for FFI and shared services
        let db_conn = self.create_database_connection().await?;
        let db_conn = Arc::new(db_conn);

        // Create certificate manager (caches root CA)
        let cert_manager = CertificateService::new_shared(&self.config.server.tls.certs_path)?;
        let cert_service = Arc::new(CertificateService::new(&self.config.server.tls.certs_path)?);

        // Create player registrar for shared player registration logic
        let player_registrar = PlayerRegistrarService::new(db_conn.clone(), cert_manager);

        // Create player identity service for cross-platform name resolution
        let identity_service = PlayerIdentityService::new(db_conn.clone());

        // Store player_registrar for FFI access
        {
            let mut pr = self.player_registrar.write()
                .map_err(|_| anyhow!("player_registrar lock poisoned"))?;
            *pr = Some(player_registrar.clone());
        }

        // Store identity_service for FFI access
        {
            let mut is = self.identity_service.write()
                .map_err(|_| anyhow!("identity_service lock poisoned"))?;
            *is = Some(identity_service.clone());
        }

        // QUIC server manager
        let mut quic_manager = QuicServerManager::new(self.config.clone());
        let webhook_receiver = quic_manager.get_webhook_receiver().clone();
        let cache_manager = quic_manager.get_cache_manager();

        // Store webhook_receiver for FFI position updates
        {
            let mut wr = self.webhook_receiver.write()
                .map_err(|_| anyhow!("webhook_receiver lock poisoned"))?;
            *wr = Some(webhook_receiver.clone());
        }

        // Create audio playback service
        let playback_cancel_token = tokio_util::sync::CancellationToken::new();
        let audio_playback_service = Arc::new(AudioPlaybackService::new(
            webhook_receiver.clone(),
            self.config.audio.file_path.clone(),
            playback_cancel_token.clone(),
            self.config.audio.max_concurrent_per_uuid,
        ));

        // Store audio_playback_service and db_conn for FFI access
        {
            let mut aps = self.audio_playback_service.write()
                .map_err(|_| anyhow!("audio_playback_service lock poisoned"))?;
            *aps = Some(audio_playback_service.clone());
        }
        {
            let mut dc = self.db_conn.write()
                .map_err(|_| anyhow!("db_conn lock poisoned"))?;
            *dc = Some(db_conn.clone());
        }

        #[cfg(feature = "bedrock")]
        let transfer_target_cache = if self.config.server.bedrock.enabled {
            Some(crate::services::bedrock::TransferTargetCache::new(
                self.config.server.bedrock.transfer_cache_ttl_secs,
            ))
        } else {
            None
        };

        // Create Rocket manager
        let mut rocket_manager = RocketManager::new(
            self.config.clone(),
            webhook_receiver,
            cache_manager,
            player_registrar,
            identity_service,
            audio_playback_service,
            cert_service,
            #[cfg(feature = "bedrock")]
            transfer_target_cache.clone(),
        );

        self.state = RuntimeState::Running;

        #[cfg(feature = "bedrock")]
        let mut dns_service = None;
        #[cfg(feature = "bedrock")]
        let mut transfer_relay = None;

        #[cfg(feature = "bedrock")]
        if self.config.server.bedrock.enabled {
            use common::traits::StreamTrait;

            let listen_ip: std::net::IpAddr = self.config.server.listen.parse()
                .unwrap_or(std::net::IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED));

            let lan_ip = if listen_ip.is_unspecified() {
                self.config.server.tls.ips.first()
                    .and_then(|ip| ip.parse().ok())
                    .unwrap_or(std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST))
            } else {
                listen_ip
            };

            let mut dns = crate::services::bedrock::DnsService::new(
                self.config.server.bedrock.dns.clone(),
                lan_ip,
            );
            if let Err(e) = dns.start().await {
                tracing::error!("Failed to start bedrock DNS service: {}", e);
            }
            dns_service = Some(dns);

            if let Some(ref cache) = transfer_target_cache {
                let mut relay = crate::services::bedrock::TransferRelayService::new(
                    self.config.server.bedrock.transfer_port,
                    cache.clone(),
                );
                if let Err(e) = relay.start().await {
                    tracing::error!("Failed to start bedrock transfer relay: {}", e);
                }
                transfer_relay = Some(relay);
            }
        }

        // Register with Meridian if configured
        if let Some(meridian_config) = &self.config.server.meridian {
            let hostname = meridian_config.host.clone()
                .or_else(|| {
                    self.config.server.tls.names.iter()
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

            if let Err(e) = service.register().await {
                tracing::error!(error = %e, "Failed to register with Meridian");
            }
        }

        let _shutdown_flag = self.shutdown_flag.clone();
        let shutdown_notify = self.shutdown_notify.clone();

        // Main event loop
        // Note: CTRL+C handling is done by the host process (Java/CLI), not here
        tokio::select! {
            result = quic_manager.start() => {
                match result {
                    Ok(_) => tracing::info!("QUIC server stopped normally"),
                    Err(e) => tracing::error!("QUIC server error: {}", e),
                }
            }
            result = rocket_manager.start() => {
                match result {
                    Ok(_) => tracing::info!("Rocket server stopped normally"),
                    Err(e) => tracing::error!("Rocket server error: {}", e),
                }
            }
            _ = shutdown_notify.notified() => {
                tracing::info!("Shutdown requested via flag, shutting down...");
            }
        }

        // Always stop QUIC regardless of which branch exited
        self.state = RuntimeState::ShuttingDown;

        #[cfg(feature = "bedrock")]
        {
            use common::traits::StreamTrait;
            if let Some(ref mut dns) = dns_service {
                if let Err(e) = dns.stop().await {
                    tracing::error!("Failed to stop bedrock DNS service: {}", e);
                }
            }
            if let Some(ref mut relay) = transfer_relay {
                if let Err(e) = relay.stop().await {
                    tracing::error!("Failed to stop bedrock transfer relay: {}", e);
                }
            }
        }

        if let Err(e) = quic_manager.stop().await {
            tracing::error!("Error stopping QUIC server: {}", e);
        }

        // This is unecessary since Rocket will stop when the main function exits
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
    pub async fn update_positions(&self, players: Vec<common::PlayerEnum>) -> Result<(), anyhow::Error> {
        let wr_guard = self.webhook_receiver.read()
            .map_err(|_| anyhow!("Failed to acquire webhook_receiver lock"))?;

        let webhook_receiver = wr_guard.as_ref()
            .ok_or_else(|| anyhow!("Server not started - webhook_receiver not available"))?;

        position_updater::PositionUpdater::broadcast_positions(players, webhook_receiver).await;
        Ok(())
    }

    /// Create a standalone database connection.
    /// This is used by the PlayerRegistrarService and can be shared between components.
    async fn create_database_connection(&self) -> Result<DatabaseConnection, anyhow::Error> {
        let dsn = self.get_dsn();
        tracing::info!("Creating standalone database connection: {}", dsn);

        let mut options = ConnectOptions::new(dsn);
        options
            .max_connections(100)
            .min_connections(1)
            .connect_timeout(std::time::Duration::from_secs(3))
            .idle_timeout(std::time::Duration::from_secs(60))
            .sqlx_logging(false);

        let conn = Database::connect(options).await?;
        Ok(conn)
    }

    /// Get the database DSN string from config.
    fn get_dsn(&self) -> String {
        match self.config.database.scheme.as_str() {
            "sqlite" | "sqlite3" => {
                let path = std::path::Path::new(&self.config.database.database);
                if !path.exists() {
                    match std::fs::File::create(&self.config.database.database) {
                        Ok(_) => {}
                        Err(_e) => {
                            panic!(
                                "Verify that {} exists and is writable. You may need to create this file.",
                                &self.config.database.database
                            );
                        }
                    }
                }
                format!("sqlite://{}", &self.config.database.database)
            }
            "mysql" => format!(
                "mysql://{}:{}@{}:{}/{}",
                &self.config.database.username.clone().unwrap_or(String::from("")),
                &self.config.database.password.clone().unwrap_or(String::from("")),
                &self.config.database.host.clone().unwrap_or(String::from("127.0.0.1")),
                &self.config.database.port.unwrap_or(3306),
                &self.config.database.database
            ),
            _ => format!("sqlite://{}", "/etc/bvc/bvc.sqlite3"),
        }
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
            tracing::Level::INFO => "info,hyper=off,rustls=off,rocket::server=off",
            tracing::Level::DEBUG => "info",
            tracing::Level::TRACE => "debug",
            tracing::Level::ERROR => "error,hyper=off,rustls=off,rocket::server=off",
            tracing::Level::WARN => "warn,hyper=off,rustls=off,rocket::server=off",
        };

        subscriber
            .with_writer(non_blocking)
            .with_max_level(self.config.get_tracing_log_level())
            .with_level(true)
            .with_line_number(&self.config.log.level == "trace")
            .with_file(&self.config.log.level == "trace")
            .with_env_filter(env_filter)
            .with_ansi(true)
            .compact()
            .init();

        self._logger_guard = Some(guard);
        Ok(())
    }

    /// Ensure the QUIC server's CA cert and key exist and that the cert's SAN
    /// extension matches the configured `tls.names + tls.ips`. The key is
    /// generated exactly once per deployment; the cert is re-signed with the
    /// same key whenever the configured SAN set drifts. Returns
    /// `(cert_pem, key_pem)`.
    async fn generate_ca(&self) -> Result<(String, String), anyhow::Error> {
        let mut san_names = self.config.server.tls.names.clone();
        san_names.append(&mut self.config.server.tls.ips.clone());
        ca_cert::CaCertManager::new(&self.config.server.tls.certs_path).ensure(&san_names)
    }
}
