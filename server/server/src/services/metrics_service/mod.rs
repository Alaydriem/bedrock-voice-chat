pub mod event;
pub mod metric;
pub mod posthog;

use std::sync::{Arc, OnceLock};
use std::time::Duration;

use chrono::Utc;
use metrics::{counter, describe_histogram, gauge, histogram};
use metrics_exporter_dogstatsd::DogStatsDBuilder;
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};
use metrics_util::layers::FanoutBuilder;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use crate::services::metrics_service::event::TelemetryEvent;
use crate::services::metrics_service::metric::Metric;
use crate::services::metrics_service::posthog::PosthogClient;

const POSTHOG_KEY: Option<&str> = option_env!("POSTHOG_KEY");
const POSTHOG_HOST: Option<&str> = option_env!("POSTHOG_HOST");
const DEFAULT_POSTHOG_HOST: &str = "https://us.i.posthog.com";
const EVENT_CHANNEL_CAPACITY: usize = 1024;

pub struct MetricsService {
    prometheus: PrometheusHandle,
    server_id: String,
    sender: Option<mpsc::Sender<TelemetryEvent>>,
    posthog_drain: Option<CancellationToken>,
}

impl MetricsService {
    // Builds the service, installs the process-global recorder (once), and spawns the
    // PostHog drain task if telemetry is enabled and a compile-time key exists. Returns
    // the shared service plus the PostHog task handle (for an awaited drain on shutdown).
    pub fn new_shared(
        telemetry_enabled: bool,
        certs_path: &str,
        server_cert_path: &str,
    ) -> (Arc<Self>, Option<JoinHandle<()>>) {
        let version = env!("CARGO_PKG_VERSION");
        let prometheus = Self::global_prometheus_handle();
        let server_id = Self::derive_server_id(certs_path);
        let hostname_sha = Self::derive_hostname_sha(server_cert_path);

        let (sender, drain, handle) = match (telemetry_enabled, POSTHOG_KEY) {
            (true, Some(key)) if !key.is_empty() => {
                let (tx, rx) = mpsc::channel::<TelemetryEvent>(EVENT_CHANNEL_CAPACITY);
                let host = POSTHOG_HOST
                    .filter(|h| !h.is_empty())
                    .unwrap_or(DEFAULT_POSTHOG_HOST)
                    .to_string();
                let client = PosthogClient::new(
                    host,
                    key.to_string(),
                    server_id.clone(),
                    version.to_string(),
                );
                let drain = CancellationToken::new();
                let drain_child = drain.clone();
                let handle = tokio::spawn(async move { client.run(rx, drain_child).await });

                // Boot ping: one server_started per process so every enabled deployment
                // reports its existence, version, and hostname even if it sees no traffic.
                let _ = tx.try_send(TelemetryEvent::ServerStarted {
                    at: Utc::now(),
                    hostname_sha,
                });
                tracing::info!("PostHog fleet telemetry enabled");
                (Some(tx), Some(drain), Some(handle))
            }
            _ => {
                tracing::info!("PostHog telemetry disabled (flag off or no compile-time key)");
                (None, None, None)
            }
        };

        let service = Arc::new(Self {
            prometheus,
            server_id,
            sender,
            posthog_drain: drain,
        });
        (service, handle)
    }

    // The metrics-rs global recorder is process-wide, install-once infrastructure (like
    // the tracing subscriber). A second runtime in the same process (embedded FFI restart)
    // reuses this install rather than orphaning a new handle. Pre-registers counters/gauges
    // at 0 so an idle server's /metrics shows them instead of "no data".
    fn global_prometheus_handle() -> PrometheusHandle {
        static GLOBAL: OnceLock<PrometheusHandle> = OnceLock::new();
        GLOBAL
            .get_or_init(|| {
                let recorder = PrometheusBuilder::new().build_recorder();
                let handle = recorder.handle();

                let mut fanout = FanoutBuilder::default().add_recorder(recorder);
                match DogStatsDBuilder::default().build() {
                    Ok(dogstatsd) => {
                        fanout = fanout.add_recorder(dogstatsd);
                        tracing::info!("statsd/dogstatsd metrics export enabled (127.0.0.1:8125)");
                    }
                    Err(e) => tracing::warn!(
                        "statsd exporter unavailable ({}); exposing Prometheus /metrics only",
                        e
                    ),
                }

                if metrics::set_global_recorder(fanout.build()).is_err() {
                    tracing::warn!("global metrics recorder already installed elsewhere");
                }

                for m in Metric::counters() {
                    counter!(m.name()).absolute(0);
                }
                gauge!(Metric::ActivePlayers.name()).set(0.0);
                gauge!(Metric::ActiveChannels.name()).set(0.0);
                gauge!(Metric::PlayersInChannels.name()).set(0.0);
                describe_histogram!(
                    Metric::SessionDurationSeconds.name(),
                    "Player voice session duration in seconds"
                );
                describe_histogram!(
                    Metric::AudioRouteDurationSeconds.name(),
                    "Per-frame route_audio_frame duration in seconds"
                );
                gauge!(Metric::BuildInfo.name(), "version" => env!("CARGO_PKG_VERSION")).set(1.0);

                handle
            })
            .clone()
    }

    // server_id groups all fleet events by deployment. The CA is required for the server
    // to run at all, so an unreadable CA is fatal — we fail loud rather than fabricate a
    // random id that would make every restart look like a brand-new deployment.
    pub fn derive_server_id(certs_path: &str) -> String {
        let ca_path = std::path::Path::new(certs_path).join("ca.crt");
        let bytes = std::fs::read(&ca_path).unwrap_or_else(|e| {
            panic!(
                "CA cert required at {} ({}); BVC cannot run without its CA",
                ca_path.display(),
                e
            )
        });
        blake3::hash(&bytes).to_hex().to_string()
    }

    // Blake3 of the server TLS cert's common name (the domain it primarily responds from).
    // Best-effort: telemetry decoration, not identity, so a parse failure logs and yields "".
    pub fn derive_hostname_sha(server_cert_path: &str) -> String {
        let pem = match std::fs::read(server_cert_path) {
            Ok(p) => p,
            Err(e) => {
                tracing::warn!("could not read server cert for hostname_sha: {}", e);
                return String::new();
            }
        };
        let cn = x509_parser::pem::parse_x509_pem(&pem)
            .ok()
            .and_then(|(_, p)| p.parse_x509().ok().map(|c| c.tbs_certificate.subject.to_string()));
        match cn {
            Some(subject) => blake3::hash(subject.as_bytes()).to_hex().to_string(),
            None => {
                tracing::warn!("could not parse server cert subject for hostname_sha");
                String::new()
            }
        }
    }

    pub fn server_id(&self) -> &str {
        &self.server_id
    }

    pub fn record_connect(&self) {
        counter!(Metric::PlayerConnectionsTotal.name()).increment(1);
        self.emit(TelemetryEvent::Connected { at: Utc::now() });
    }

    pub fn record_disconnect(&self, duration: Duration) {
        counter!(Metric::PlayerDisconnectionsTotal.name()).increment(1);
        histogram!(Metric::SessionDurationSeconds.name()).record(duration.as_secs_f64());
        self.emit(TelemetryEvent::Disconnected {
            at: Utc::now(),
            duration_secs: duration.as_secs(),
        });
    }

    pub fn record_channel_join(&self) {
        counter!(Metric::ChannelJoinsTotal.name()).increment(1);
        self.emit(TelemetryEvent::ChannelJoined { at: Utc::now() });
    }

    pub fn record_channel_leave(&self) {
        counter!(Metric::ChannelLeavesTotal.name()).increment(1);
        self.emit(TelemetryEvent::ChannelLeft { at: Utc::now() });
    }

    // Per-frame routing cost on the audio hot path. Counter + histogram record
    // is lock-free and nanosecond-scale, three orders of magnitude below the
    // route work itself. No PostHog event: per-frame volume, no fleet value.
    pub fn record_audio_route(&self, duration: Duration) {
        counter!(Metric::AudioFramesRoutedTotal.name()).increment(1);
        histogram!(Metric::AudioRouteDurationSeconds.name()).record(duration.as_secs_f64());
    }

    // A recipient's bounded output queue was full and the frame was dropped for
    // them — the first user-audible routing failure mode under load.
    pub fn record_audio_route_drop(&self) {
        counter!(Metric::AudioRouteRecipientDropsTotal.name()).increment(1);
    }

    pub fn set_active_players(&self, value: i64) {
        gauge!(Metric::ActivePlayers.name()).set(value as f64);
    }

    pub fn set_active_channels(&self, value: i64) {
        gauge!(Metric::ActiveChannels.name()).set(value as f64);
    }

    pub fn set_players_in_channels(&self, value: i64) {
        gauge!(Metric::PlayersInChannels.name()).set(value as f64);
    }

    pub fn render(&self) -> String {
        self.prometheus.render()
    }

    // Signals the PostHog task to flush and exit; the caller awaits its JoinHandle.
    pub fn begin_posthog_drain(&self) {
        if let Some(drain) = &self.posthog_drain {
            drain.cancel();
        }
    }

    fn emit(&self, event: TelemetryEvent) {
        if let Some(tx) = &self.sender {
            let _ = tx.try_send(event);
        }
    }
}
