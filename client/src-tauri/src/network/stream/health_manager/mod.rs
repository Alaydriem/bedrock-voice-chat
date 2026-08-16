use bytes::Bytes;
use common::consts::version::PROTOCOL_VERSION;
use common::response::{ApiConfigCheckResponse, ApiConfigResponse};
use crate::network::stream::HealthPublisher;
use crate::network::stream::link::DatagramLink;
use common::structs::network::ConnectionHealth;
use common::structs::packet::{
    HealthCheckPacket, PacketType, QuicNetworkPacket, QuicNetworkPacketData,
};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tauri::Emitter;
use tokio::task::AbortHandle;

use super::stream_manager::HealthMonitorState;
use crate::api::EndpointBreaker;

mod health_config;
mod probe_result;
mod reconnect_config;

pub use health_config::HealthConfig;
use probe_result::ProbeResult;
pub use reconnect_config::ReconnectConfig;

/// Manages connection health monitoring and automatic reconnection
pub struct ConnectionHealthManager {
    /// Records a QUIC session that connected and then stopped carrying traffic — the one
    /// failure a reachability probe cannot see, because the handshake it measures still
    /// succeeds on exactly the networks that produce it.
    transport_verdict: Arc<crate::network::TransportVerdict>,
    health_state: Arc<HealthMonitorState>,
    shutdown: Arc<AtomicBool>,
    task_handle: Option<AbortHandle>,
    app_handle: tauri::AppHandle,
    health_config: HealthConfig,
    reconnect_config: ReconnectConfig,
}

impl ConnectionHealthManager {
    /// Create a new ConnectionHealthManager
    pub fn new(
        app_handle: tauri::AppHandle,
        transport_verdict: Arc<crate::network::TransportVerdict>,
    ) -> Self {
        Self {
            transport_verdict,
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
    pub fn start(&mut self, link: DatagramLink, server_url: String) {
        self.stop();
        self.shutdown.store(false, Ordering::Relaxed);

        HealthPublisher::publish(&self.app_handle, ConnectionHealth::Connected);

        let health_state = self.health_state.clone();
        let shutdown = self.shutdown.clone();
        let app_handle = self.app_handle.clone();
        let health_config = self.health_config.clone();
        let reconnect_config = self.reconnect_config.clone();
        let transport_verdict = self.transport_verdict.clone();

        let handle = tokio::spawn(async move {
            Self::run_health_monitor(
                health_state,
                link,
                transport_verdict,
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
        link: DatagramLink,
        transport_verdict: Arc<crate::network::TransportVerdict>,
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

            // Terminal: the server refused this identity, so probing and re-dialing
            // would loop forever against a server that is perfectly healthy.
            if health_state.has_unauthorized() {
                log::error!(
                    "Server refused this connection's identity; stopping reconnect attempts"
                );
                HealthPublisher::publish(
                    &app_handle,
                    ConnectionHealth::Unauthorized {
                        reason: "The server refused this connection's identity. Your \
                                 credentials may have been revoked — sign in again."
                            .to_string(),
                    },
                );
                break;
            }

            if health_state.has_protocol_error() {
                log::error!(
                    "Datagram decode failures indicate an incompatible server protocol; tearing down connection"
                );
                Self::emit_version_mismatch(&server_url, &app_handle).await;
                break;
            }

            if health_state.should_send_health_check(health_config.threshold) {
                log::trace!("Sending health check packet");

                Self::send_health_check(&link, &health_state).await;
                tokio::time::sleep(health_config.timeout).await;

                let failures = health_state.on_timeout();
                if failures >= health_config.max_failures {
                    log::warn!(
                        "Health check failed {} times, triggering reconnect",
                        failures
                    );

                    // The session handshook and then stopped carrying traffic. On QUIC
                    // that is the signature of a network which inspects or degrades UDP
                    // rather than blocking it, and re-probing would report the server
                    // reachable and hand back another session that dies the same way.
                    if matches!(link, DatagramLink::Quic(_)) {
                        transport_verdict.demote(&server_url);
                    }

                    Self::probe_and_reconnect(&server_url, &app_handle, &reconnect_config).await;
                    break;
                } else if failures > 0 {
                    log::debug!("Health check timeout, failure count: {}", failures);
                }
            }
        }
    }

    /// Send a health check packet
    async fn send_health_check(link: &DatagramLink, health_state: &HealthMonitorState) {
        let health_packet = QuicNetworkPacket {
            packet_type: PacketType::HealthCheck,
            data: QuicNetworkPacketData::HealthCheck(HealthCheckPacket),
                    // Not a server fan-out, so this envelope carries no sequence.
            ..Default::default()
        };

        health_state.set_awaiting(true);

        if let Ok(bytes) = health_packet.to_datagram()
            && let Err(e) = link.send(Bytes::from(bytes))
        {
            log::warn!("Failed to send health check packet: {}", e);
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

        HealthPublisher::publish(app_handle, ConnectionHealth::Disconnected);

        while attempt < config.max_attempts {
            HealthPublisher::publish(app_handle, ConnectionHealth::Reconnecting { attempt });

            match Self::probe_server(server_url).await {
                ProbeResult::Available => {
                    log::info!("Server is back online, triggering refresh...");
                    let _ = app_handle.emit("trigger_refresh", ());
                    return;
                }
                ProbeResult::VersionMismatch {
                    client_version,
                    server_version,
                    client_too_old,
                } => {
                    log::error!(
                        "Protocol version mismatch detected: client={}, server={}, client_too_old={}",
                        client_version,
                        server_version,
                        client_too_old
                    );
                    HealthPublisher::publish(
                        app_handle,
                        ConnectionHealth::VersionMismatch {
                            client_version,
                            server_version,
                            client_too_old,
                        },
                    );
                    // Exit early - don't keep retrying on version mismatch
                    return;
                }
                ProbeResult::Unavailable => {
                    log::warn!("Server not yet available (attempt {}), waiting...", attempt);
                }
            }

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
        HealthPublisher::publish(app_handle, ConnectionHealth::Failed);
    }

    /// Emit a VersionMismatch event for a connection that has been proven
    /// incompatible by datagram decode failures. The server version is fetched
    /// from `/api/config` for diagnostics; the mismatch is emitted regardless of
    /// what the semantic version comparison reports, because the decode failure
    /// is already authoritative proof the wire formats differ.
    async fn emit_version_mismatch(server_url: &str, app_handle: &tauri::AppHandle) {
        let (server_version, client_too_old) = Self::fetch_server_version(server_url).await;
        log::warn!(
            "Protocol mismatch confirmed: client={}, server={}, client_too_old={}",
            PROTOCOL_VERSION,
            server_version,
            client_too_old
        );
        HealthPublisher::publish(
            app_handle,
            ConnectionHealth::VersionMismatch {
                client_version: PROTOCOL_VERSION.to_string(),
                server_version,
                client_too_old,
            },
        );
    }

    /// Fetch the server's advertised protocol version from `/api/config`,
    /// returning the version string and whether this client is the older side.
    /// Falls back to `("unknown", false)` when the config cannot be read.
    async fn fetch_server_version(server_url: &str) -> (String, bool) {
        #[allow(unused_mut)]
        let mut builder = common::reqwest::Client::builder().timeout(Duration::from_secs(5));

        #[cfg(dev)]
        {
            builder = builder.danger_accept_invalid_certs(true);
        }

        let client = match builder.build() {
            Ok(c) => c,
            Err(_) => return ("unknown".to_string(), false),
        };

        let base_url = if server_url.starts_with("http://") || server_url.starts_with("https://") {
            server_url.to_string()
        } else {
            format!("https://{}", server_url)
        };

        let url = format!("{}/api/config", base_url);

        let body = match client.get(&url).send().await {
            Ok(resp) if resp.status().is_success() => match resp.text().await {
                Ok(text) => text,
                Err(_) => return ("unknown".to_string(), false),
            },
            _ => return ("unknown".to_string(), false),
        };

        match serde_json::from_str::<ApiConfigResponse>(&body) {
            Ok(config) => {
                let server_version = config.protocol_version.clone();
                let check = ApiConfigCheckResponse::from_config(config, PROTOCOL_VERSION);
                (server_version, check.client_too_old)
            }
            Err(_) => ("unknown".to_string(), false),
        }
    }

    /// Probe the server's HTTP endpoint to check availability and version compatibility
    async fn probe_server(server_url: &str) -> ProbeResult {
        #[allow(unused_mut)]
        let mut builder = common::reqwest::Client::builder().timeout(Duration::from_secs(5));

        #[cfg(dev)]
        {
            builder = builder.danger_accept_invalid_certs(true);
        }

        let client = match builder.build() {
            Ok(c) => c,
            Err(e) => {
                log::warn!("Failed to build HTTP client for probe: {}", e);
                return ProbeResult::Unavailable;
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

                // This request carried no credentials and went nowhere near the
                // endpoint's breaker, so reaching the server here is evidence the
                // breaker does not have. Recorded against `server_url` rather than the
                // normalised `base_url`, because the pooled `Api` is keyed by the
                // string the webview passes and that is the one being unblocked.
                EndpointBreaker::note_reachable(server_url);

                if !resp.status().is_success() {
                    return ProbeResult::Unavailable;
                }

                let body = match resp.text().await {
                    Ok(text) => text,
                    Err(e) => {
                        log::warn!("Failed to read response body: {}", e);
                        return ProbeResult::Unavailable;
                    }
                };

                match serde_json::from_str::<ApiConfigResponse>(&body) {
                    Ok(config) => {
                        let server_version = config.protocol_version.clone();
                        let check = ApiConfigCheckResponse::from_config(config, PROTOCOL_VERSION);
                        if !check.compatible {
                            log::warn!(
                                "Protocol version mismatch: client={}, server={}, client_too_old={}",
                                check.client_version,
                                server_version,
                                check.client_too_old,
                            );
                            return ProbeResult::VersionMismatch {
                                client_version: check.client_version,
                                server_version,
                                client_too_old: check.client_too_old,
                            };
                        }
                        ProbeResult::Available
                    }
                    Err(_) => {
                        #[derive(serde::Deserialize)]
                        struct LegacyApiConfig {
                            status: String,
                            #[allow(dead_code)]
                            client_id: String,
                        }

                        match serde_json::from_str::<LegacyApiConfig>(&body) {
                            Ok(legacy) if legacy.status == "Ok" => {
                                log::warn!(
                                    "Server is running outdated version without protocol_version field"
                                );
                                ProbeResult::VersionMismatch {
                                    client_version: PROTOCOL_VERSION.to_string(),
                                    server_version: "unknown (outdated)".to_string(),
                                    client_too_old: false,
                                }
                            }
                            _ => {
                                log::warn!("Failed to parse ApiConfigResponse response");
                                ProbeResult::Unavailable
                            }
                        }
                    }
                }
            }
            Err(e) => {
                log::debug!("Probe failed: {}", e);
                ProbeResult::Unavailable
            }
        }
    }
}
