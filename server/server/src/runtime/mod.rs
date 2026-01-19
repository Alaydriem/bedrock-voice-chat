pub mod position_updater;

use crate::config::ApplicationConfig;
use crate::rs::manager::RocketManager;
use crate::stream::quic::{QuicServerManager, WebhookReceiver};

use anyhow::anyhow;
use common::structs::channel::Channel;
use faccess::PathExt;
use rcgen::{
    CertificateParams, DistinguishedName, ExtendedKeyUsagePurpose, IsCa, KeyPair, KeyUsagePurpose,
};
use rocket::time::{Duration, OffsetDateTime};
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use tracing::info;
use tracing_appender::non_blocking::WorkerGuard;

/// Runtime state for the server
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeState {
    /// Server is not started
    Stopped,
    /// Server is starting up
    Starting,
    /// Server is running
    Running,
    /// Server is shutting down
    ShuttingDown,
}

/// Server runtime that manages the full BVC server stack.
/// This is the main entry point for both CLI and FFI usage.
pub struct ServerRuntime {
    config: ApplicationConfig,
    state: RuntimeState,
    shutdown_flag: Arc<AtomicBool>,
    /// Webhook receiver for sending position updates directly (populated after start)
    webhook_receiver: Arc<RwLock<Option<WebhookReceiver>>>,
    _logger_guard: Option<WorkerGuard>,
}

impl ServerRuntime {
    /// Create a new runtime with ApplicationConfig
    pub fn new(config: ApplicationConfig) -> Result<Self, anyhow::Error> {
        Ok(Self {
            config,
            state: RuntimeState::Stopped,
            shutdown_flag: Arc::new(AtomicBool::new(false)),
            webhook_receiver: Arc::new(RwLock::new(None)),
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

    /// Initialize and start the server (async)
    pub async fn start_async(&mut self) -> Result<(), anyhow::Error> {
        if self.state != RuntimeState::Stopped {
            return Err(anyhow!("Server is already running or starting"));
        }

        self.state = RuntimeState::Starting;

        // Setup logging
        self.setup_logging()?;

        info!("Logger established!");

        // Generate CA certificates
        self.generate_ca().await?;

        // State cache for recording groups a player is in
        let channel_cache = Arc::new(async_mutex::Mutex::new(
            moka::future::Cache::<String, Channel>::builder()
                .max_capacity(100)
                .build(),
        ));

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

        // Create Rocket manager
        let rocket_manager = RocketManager::new(
            self.config.clone(),
            webhook_receiver,
            channel_cache.clone(),
            cache_manager,
        );

        self.state = RuntimeState::Running;

        let shutdown_flag = self.shutdown_flag.clone();

        // Main event loop - responds to shutdown flag only
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
            _ = async {
                loop {
                    if shutdown_flag.load(Ordering::SeqCst) {
                        break;
                    }
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                }
            } => {
                tracing::info!("Shutdown requested via flag, shutting down...");
            }
        }

        // Always stop QUIC regardless of which branch exited
        self.state = RuntimeState::ShuttingDown;
        if let Err(e) = quic_manager.stop().await {
            tracing::error!("Error stopping QUIC server: {}", e);
        }

        self.state = RuntimeState::Stopped;
        Ok(())
    }

    /// Signal the server to stop gracefully
    pub fn request_shutdown(&self) {
        self.shutdown_flag.store(true, Ordering::SeqCst);
    }

    /// Get a clone of the webhook receiver Arc for external use
    pub fn get_webhook_receiver(&self) -> Arc<RwLock<Option<WebhookReceiver>>> {
        self.webhook_receiver.clone()
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

        position_updater::broadcast_positions(players, webhook_receiver).await;
        Ok(())
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
            "stdout" => {
                (non_blocking, guard) = tracing_appender::non_blocking(std::io::stdout());
            }
            "callback" => {
                // For FFI mode - logging goes to callback (handled separately)
                // For now, just use stdout
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
            .compact()
            .init();

        self._logger_guard = Some(guard);
        Ok(())
    }

    /// Generate the root CA certificates for QUIC mTLS
    async fn generate_ca(&self) -> Result<(String, String), anyhow::Error> {
        let certs_path = &self.config.server.tls.certs_path;
        let root_ca_path_str = format!("{}/ca.crt", certs_path);
        let root_ca_key_path_str = format!("{}/ca.key", certs_path);

        // If the certificates already exist, just return them
        if Path::new(&root_ca_key_path_str).exists() {
            return Ok((
                std::fs::read_to_string(&root_ca_path_str)?,
                std::fs::read_to_string(&root_ca_key_path_str)?,
            ));
        }

        let cert_root_path = Path::new(certs_path);
        if !cert_root_path.exists() {
            std::fs::create_dir_all(cert_root_path).map_err(|_| {
                anyhow!(
                    "Could not create directory {}",
                    cert_root_path.to_string_lossy()
                )
            })?;
        }

        // Create the root CA certificate
        let root_kp = KeyPair::generate().map_err(|_| {
            anyhow!(
                "Unable to generate root key. Check the certs_path configuration variable to ensure the path is writable"
            )
        })?;

        let mut distinguished_name = DistinguishedName::new();
        distinguished_name.push(rcgen::DnType::CommonName, "Bedrock Voice Chat");

        let mut san_names = self.config.server.tls.names.clone();
        san_names.append(&mut self.config.server.tls.ips.clone());

        let root_certificate = CertificateParams::new(san_names)
            .map_err(|_| {
                anyhow!(
                    "Unable to generate root certificates. Check the certs_path configuration variable"
                )
            })
            .and_then(|mut ca_params| {
                ca_params.is_ca = IsCa::NoCa;
                ca_params.not_before = OffsetDateTime::now_utc()
                    .checked_sub(Duration::days(3))
                    .unwrap();
                ca_params.distinguished_name = distinguished_name;
                ca_params.use_authority_key_identifier_extension = true;
                ca_params.key_usages = vec![KeyUsagePurpose::KeyCertSign];
                ca_params.extended_key_usages = vec![
                    ExtendedKeyUsagePurpose::ClientAuth,
                    ExtendedKeyUsagePurpose::ServerAuth,
                ];
                ca_params.self_signed(&root_kp).map_err(|e| anyhow!(e))
            })?;

        let cert = root_certificate.pem();
        let key = root_kp.serialize_pem();

        let mut cert_file = File::create(&root_ca_path_str)?;
        cert_file.write_all(cert.as_bytes())?;
        let mut key_file = File::create(&root_ca_key_path_str)?;
        key_file.write_all(key.as_bytes())?;

        info!("Generated CA certificates at {}", certs_path);

        Ok((cert, key))
    }
}
