use bytes::Bytes;
use common::s2n_quic::Connection;
use common::structs::network::ConnectionHealth;
use common::structs::packet::{
    HealthCheckPacket, PacketOwner, PacketType, QuicNetworkPacket, QuicNetworkPacketData,
};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tauri::Emitter;
use tokio::task::AbortHandle;

use super::stream_manager::HealthMonitorState;

/// Configuration for health monitoring
#[derive(Debug, Clone)]
pub struct HealthConfig {
    /// How often to check if we need to send a health check
    pub check_interval: Duration,
    /// Send health check if no packets received for this duration
    pub threshold: Duration,
    /// How long to wait for health check response
    pub timeout: Duration,
    /// Number of consecutive failures before triggering reconnect
    pub max_failures: u32,
}

impl Default for HealthConfig {
    fn default() -> Self {
        Self {
            check_interval: Duration::from_secs(2),
            threshold: Duration::from_secs(5),
            timeout: Duration::from_secs(2),
            max_failures: 3,
        }
    }
}

/// Configuration for reconnection probing
#[derive(Debug, Clone)]
pub struct ReconnectConfig {
    /// Initial delay before first probe
    pub initial_delay: Duration,
    /// Maximum delay between probes
    pub max_delay: Duration,
    /// Multiplier for exponential backoff
    pub backoff_multiplier: f64,
    /// Jitter factor (0.0 to 1.0)
    pub jitter_factor: f64,
    /// Maximum number of probe attempts
    pub max_attempts: u32,
}

impl Default for ReconnectConfig {
    fn default() -> Self {
        Self {
            initial_delay: Duration::from_millis(1_000),
            max_delay: Duration::from_millis(10_000),
            backoff_multiplier: 2.0,
            jitter_factor: 0.2,
            max_attempts: 20,
        }
    }
}

/// Manages connection health monitoring and automatic reconnection
pub struct ConnectionHealthManager {
    health_state: Arc<HealthMonitorState>,
    shutdown: Arc<AtomicBool>,
    task_handle: Option<AbortHandle>,
    app_handle: tauri::AppHandle,
    health_config: HealthConfig,
    reconnect_config: ReconnectConfig,
}

impl ConnectionHealthManager {
    /// Create a new ConnectionHealthManager
    pub fn new(app_handle: tauri::AppHandle) -> Self {
        Self {
            health_state: Arc::new(HealthMonitorState::new()),
            shutdown: Arc::new(AtomicBool::new(false)),
            task_handle: None,
            app_handle,
            health_config: HealthConfig::default(),
            reconnect_config: ReconnectConfig::default(),
        }
    }

    /// Get the health state for sharing with input stream
    pub fn health_state(&self) -> Arc<HealthMonitorState> {
        self.health_state.clone()
    }

    /// Reset the health state (e.g., on new connection)
    pub fn reset(&self) {
        self.health_state.reset();
    }

    /// Start health monitoring for a connection
    pub fn start(
        &mut self,
        connection: Arc<Connection>,
        packet_owner: Option<PacketOwner>,
        server_url: String,
    ) {
        self.stop();
        self.shutdown.store(false, Ordering::Relaxed);

        let _ = self
            .app_handle
            .emit("connection_health", ConnectionHealth::Connected);

        let health_state = self.health_state.clone();
        let shutdown = self.shutdown.clone();
        let app_handle = self.app_handle.clone();
        let health_config = self.health_config.clone();
        let reconnect_config = self.reconnect_config.clone();

        let handle = tokio::spawn(async move {
            Self::run_health_monitor(
                health_state,
                connection,
                packet_owner,
                shutdown,
                app_handle,
                server_url,
                health_config,
                reconnect_config,
            )
            .await;
        });

        self.task_handle = Some(handle.abort_handle());
    }

    /// Stop health monitoring
    pub fn stop(&mut self) {
        self.shutdown.store(true, Ordering::Relaxed);
        if let Some(handle) = self.task_handle.take() {
            handle.abort();
        }
    }

    /// Main health monitor loop
    async fn run_health_monitor(
        health_state: Arc<HealthMonitorState>,
        connection: Arc<Connection>,
        packet_owner: Option<PacketOwner>,
        shutdown: Arc<AtomicBool>,
        app_handle: tauri::AppHandle,
        server_url: String,
        health_config: HealthConfig,
        reconnect_config: ReconnectConfig,
    ) {
        let mut interval = tokio::time::interval(health_config.check_interval);

        loop {
            interval.tick().await;

            if shutdown.load(Ordering::Relaxed) {
                log::debug!("Health monitor shutting down");
                break;
            }

            if health_state.should_send_health_check(health_config.threshold) {
                log::trace!("Sending health check packet");

                Self::send_health_check(&connection, &packet_owner, &health_state).await;
                tokio::time::sleep(health_config.timeout).await;

                let failures = health_state.on_timeout();
                if failures >= health_config.max_failures {
                    log::warn!(
                        "Health check failed {} times, triggering reconnect",
                        failures
                    );
                    Self::probe_and_reconnect(&server_url, &app_handle, &reconnect_config).await;
                    break;
                } else if failures > 0 {
                    log::debug!("Health check timeout, failure count: {}", failures);
                }
            }
        }
    }

    /// Send a health check packet
    async fn send_health_check(
        connection: &Connection,
        packet_owner: &Option<PacketOwner>,
        health_state: &HealthMonitorState,
    ) {
        let health_packet = QuicNetworkPacket {
            packet_type: PacketType::HealthCheck,
            owner: packet_owner.clone(),
            data: QuicNetworkPacketData::HealthCheck(HealthCheckPacket),
        };

        health_state.set_awaiting(true);

        if let Ok(bytes) = health_packet.to_datagram() {
            let send_result = connection.datagram_mut(
                |dg: &mut common::s2n_quic::provider::datagram::default::Sender| {
                    dg.send_datagram(Bytes::from(bytes))
                },
            );

            if let Err(e) = send_result {
                log::warn!("Failed to send health check packet: {}", e);
            }
        }
    }

    /// Probe server availability and trigger refresh when ready
    async fn probe_and_reconnect(
        server_url: &str,
        app_handle: &tauri::AppHandle,
        config: &ReconnectConfig,
    ) {
        let mut attempt = 0u32;
        let mut delay = config.initial_delay;

        let _ = app_handle.emit("connection_health", ConnectionHealth::Disconnected);

        while attempt < config.max_attempts {
            let _ = app_handle.emit(
                "connection_health",
                ConnectionHealth::Reconnecting { attempt },
            );

            if Self::probe_server(server_url).await {
                log::info!("Server is back online, triggering refresh...");
                let _ = app_handle.emit("trigger_refresh", ());
                return;
            }

            log::warn!("Server not yet available (attempt {}), waiting...", attempt);
            attempt += 1;

            let jitter = rand::random::<f64>() * config.jitter_factor * 2.0 - config.jitter_factor;
            let delay_with_jitter = delay.as_secs_f64() * (1.0 + jitter);
            tokio::time::sleep(Duration::from_secs_f64(delay_with_jitter)).await;

            delay = Duration::from_millis(
                ((delay.as_millis() as f64 * config.backoff_multiplier) as u64)
                    .min(config.max_delay.as_millis() as u64),
            );
        }

        log::error!("Failed to reconnect after {} attempts", config.max_attempts);
        let _ = app_handle.emit("connection_health", ConnectionHealth::Failed);
    }

    /// Probe the server's HTTP endpoint to check availability
    async fn probe_server(server_url: &str) -> bool {
        let client = match reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .danger_accept_invalid_certs(true)
            .build()
        {
            Ok(c) => c,
            Err(e) => {
                log::warn!("Failed to build HTTP client for probe: {}", e);
                return false;
            }
        };

        let base_url = if server_url.starts_with("http://") || server_url.starts_with("https://") {
            server_url.to_string()
        } else {
            format!("https://{}", server_url)
        };

        let url = format!("{}/api/config", base_url);
        log::debug!("Probing server at: {}", url);

        match client.get(&url).send().await {
            Ok(resp) => {
                log::debug!("Probe response status: {}", resp.status());
                resp.status().is_success()
            }
            Err(e) => {
                log::debug!("Probe failed: {}", e);
                false
            }
        }
    }
}
