pub mod event;
pub mod heartbeat_snapshot;
pub mod host_capability;
pub mod interaction;
pub mod metric;
pub mod posthog;

use std::io::ErrorKind;
use std::net::{SocketAddr, UdpSocket};
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

use chrono::Utc;
use common::structs::metrics::TransportKind;
use metrics::{counter, describe_histogram, gauge, histogram};
use metrics_exporter_dogstatsd::DogStatsDBuilder;
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};
use metrics_util::layers::FanoutBuilder;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use crate::services::metrics_service::event::TelemetryEvent;
use crate::services::metrics_service::heartbeat_snapshot::HeartbeatSnapshot;
use crate::services::metrics_service::host_capability::HostCapability;
use crate::services::metrics_service::interaction::InteractionRoute;
use crate::services::metrics_service::interaction::InteractionTracker;
use crate::services::metrics_service::metric::Metric;
use crate::services::metrics_service::posthog::PosthogClient;

const POSTHOG_KEY: Option<&str> = option_env!("POSTHOG_KEY");
const POSTHOG_HOST: Option<&str> = option_env!("POSTHOG_HOST");
const DEFAULT_POSTHOG_HOST: &str = "https://us.i.posthog.com";
const EVENT_CHANNEL_CAPACITY: usize = 1024;
const STATSD_ADDR: &str = "127.0.0.1:8125";
const STATSD_PROBE_TIMEOUT: Duration = Duration::from_millis(150);
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(15 * 60);
const RECONNECT_WINDOW: Duration = Duration::from_secs(30 * 60);
const RECONNECT_CACHE_CAPACITY: u64 = 4096;

pub struct MetricsService {
    prometheus: PrometheusHandle,
    server_id: String,
    sender: Option<mpsc::Sender<TelemetryEvent>>,
    posthog_drain: Option<CancellationToken>,
    // Mirrors of the same values written to the Prometheus gauges. metrics-rs
    // exposes no read path, and the heartbeat must report the same number the
    // gauge carries, so both are written by the one method below.
    active_players: AtomicI64,
    peak_players: AtomicI64,
    interactions: InteractionTracker,
    started_at: Instant,
    features_enabled: Vec<String>,
    recording_enabled: bool,
    // Sampled at beat time rather than pushed on change: the write paths that set a
    // player's recording flag are several, and a counter hooked into each would drift.
    player_state: Option<crate::stream::quic::PlayerStateCache>,
    // Recently disconnected players, so a return inside the window is reported as a
    // reconnect rather than a fresh session. The name is a local key and never
    // leaves the process — only the elapsed delta is emitted.
    recent_disconnects: moka::sync::Cache<String, Instant>,
}

impl MetricsService {
    // Builds the service, installs the process-global recorder (once), and spawns the
    // PostHog drain task if telemetry is enabled and a compile-time key exists. Returns
    // the shared service plus the PostHog task handle (for an awaited drain on shutdown).
    pub fn new_shared(
        telemetry_enabled: bool,
        certs_path: &str,
        server_cert_path: &str,
        features_enabled: Vec<String>,
        ca_minted: bool,
        recording_enabled: bool,
        player_state: Option<crate::stream::quic::PlayerStateCache>,
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
                    hostname_sha.clone(),
                );
                let drain = CancellationToken::new();
                let drain_child = drain.clone();
                let handle = tokio::spawn(async move { client.run(rx, drain_child).await });

                // Boot ping: one Server::Started per process so every enabled deployment
                // reports its existence and version even if it sees no traffic.
                let _ = tx.try_send(TelemetryEvent::ServerStarted { at: Utc::now() });

                // The CA keypair is minted exactly once per deployment, so this boot
                // creating it is the first time this server has ever run.
                if ca_minted {
                    let _ = tx.try_send(TelemetryEvent::FirstSeen { at: Utc::now() });
                }
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
            active_players: AtomicI64::new(0),
            peak_players: AtomicI64::new(0),
            interactions: InteractionTracker::new(),
            started_at: Instant::now(),
            features_enabled,
            recording_enabled,
            player_state,
            recent_disconnects: moka::sync::Cache::builder()
                .time_to_live(RECONNECT_WINDOW)
                .max_capacity(RECONNECT_CACHE_CAPACITY)
                .build(),
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
                if Self::statsd_agent_reachable(STATSD_ADDR) {
                    match DogStatsDBuilder::default()
                        .with_remote_address(STATSD_ADDR)
                        .and_then(|b| b.build())
                    {
                        Ok(dogstatsd) => {
                            fanout = fanout.add_recorder(dogstatsd);
                            tracing::info!("statsd/dogstatsd metrics export enabled ({STATSD_ADDR})");
                        }
                        Err(e) => tracing::warn!(
                            "statsd exporter unavailable ({e}); exposing Prometheus /metrics only"
                        ),
                    }
                } else {
                    tracing::warn!(
                        "no statsd agent reachable at {STATSD_ADDR}; statsd export disabled (Prometheus /metrics still available)"
                    );
                }

                if metrics::set_global_recorder(fanout.build()).is_err() {
                    tracing::warn!("global metrics recorder already installed elsewhere");
                }

                for m in Metric::counters() {
                    counter!(m.name()).absolute(0);
                }
                gauge!(Metric::ActivePlayers.name()).set(0.0);
                gauge!(Metric::PeakPlayers.name()).set(0.0);
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
                describe_histogram!(
                    Metric::PositionDatagramBytes.name(),
                    "Encoded size of each position datagram, against MAX_DATAGRAM_SIZE"
                );
                gauge!(Metric::BuildInfo.name(), "version" => env!("CARGO_PKG_VERSION")).set(1.0);

                handle
            })
            .clone()
    }

    // One-shot startup probe for a statsd/dogstatsd agent. A connected UDP socket
    // to a *rejecting* port surfaces the port-unreachable ICMP as a connection
    // error on a following syscall — the origin of the forwarder's repeated
    // "Failed to send payload" errors. Probing once here lets a hard-blocked port
    // disable statsd with a single warning instead of erroring every flush. A
    // silently dropped or genuinely idle port yields no ICMP, so it reads as
    // reachable and statsd stays on (a real agent harmlessly discards the probe).
    fn statsd_agent_reachable(addr: &str) -> bool {
        let target: SocketAddr = match addr.parse() {
            Ok(a) => a,
            Err(_) => return false,
        };
        let bind = if target.is_ipv4() { "0.0.0.0:0" } else { "[::]:0" };
        let socket = match UdpSocket::bind(bind) {
            Ok(s) => s,
            Err(_) => return false,
        };
        if socket.connect(target).is_err() {
            return false;
        }
        let _ = socket.set_read_timeout(Some(STATSD_PROBE_TIMEOUT));

        let _ = socket.send(&[]);
        let mut buf = [0u8; 8];
        if Self::is_unreachable(socket.recv(&mut buf)) {
            return false;
        }
        !Self::is_unreachable(socket.send(&[]))
    }

    // The refusal arrives as ConnectionRefused on Unix and ConnectionReset on
    // Windows; either means the port actively rejected the datagram.
    fn is_unreachable<T>(result: std::io::Result<T>) -> bool {
        matches!(
            result,
            Err(e) if matches!(e.kind(), ErrorKind::ConnectionRefused | ErrorKind::ConnectionReset)
        )
    }

    // server_id groups all fleet events by deployment. Hashes the CA's public key
    // rather than the cert file: ca.crt is re-signed whenever the configured SAN set
    // drifts, but the keypair behind it is generated once and never replaced. The CA
    // is required for the server to run at all, so an unreadable or unparseable CA is
    // fatal — we fail loud rather than fabricate an id that would make every restart
    // look like a brand-new deployment.
    //
    // This changed derivation from an earlier hash of the ca.crt file bytes, so every
    // deployment predating it reports a new id once and its PostHog history splits at
    // that point. Accepted deliberately rather than bridged with a $create_alias: the
    // very re-signing this fixes means some deployments had already forked, so a merge
    // could not have been complete either.
    pub fn derive_server_id(certs_path: &str) -> String {
        let ca_path = std::path::Path::new(certs_path).join("ca.crt");
        let bytes = std::fs::read(&ca_path).unwrap_or_else(|e| {
            panic!(
                "CA cert required at {} ({}); BVC cannot run without its CA",
                ca_path.display(),
                e
            )
        });
        let spki = x509_parser::pem::parse_x509_pem(&bytes)
            .ok()
            .and_then(|(_, pem)| pem.parse_x509().ok().map(|c| c.public_key().raw.to_vec()));
        let spki = spki.unwrap_or_else(|| {
            panic!(
                "CA cert at {} could not be parsed; BVC cannot run without a valid CA",
                ca_path.display()
            )
        });
        blake3::hash(&spki).to_hex().to_string()
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
        // The CN alone, not the whole subject DN: the value is meant to identify the
        // hostname the server answers on, and a DN string also carries O/OU/C, which
        // would change the hash on an unrelated cert-metadata edit.
        let cn = x509_parser::pem::parse_x509_pem(&pem).ok().and_then(|(_, p)| {
            p.parse_x509().ok().and_then(|c| {
                c.tbs_certificate
                    .subject
                    .iter_common_name()
                    .next()
                    .and_then(|attr| attr.as_str().ok())
                    .map(str::to_string)
            })
        });
        match cn {
            Some(cn) => blake3::hash(cn.as_bytes()).to_hex().to_string(),
            None => {
                tracing::warn!("could not parse server cert common name for hostname_sha");
                String::new()
            }
        }
    }

    pub fn server_id(&self) -> &str {
        &self.server_id
    }

    /// `transport` is a label rather than a separate metric so one query answers both
    /// "how many players are connected" and "how many of them could not use QUIC". The
    /// second question has no client-side answer an operator can see.
    pub fn record_connect(&self, player_name: &str, transport: TransportKind) {
        counter!(Metric::PlayerConnectionsTotal.name(), "transport" => transport.as_str())
            .increment(1);
        match self.recent_disconnects.get(player_name) {
            Some(left_at) => {
                self.recent_disconnects.invalidate(player_name);
                self.emit(TelemetryEvent::PlayerReconnected {
                    at: Utc::now(),
                    time_since_disconnect_secs: left_at.elapsed().as_secs(),
                });
            }
            None => self.emit(TelemetryEvent::PlayerConnected { at: Utc::now() }),
        }
    }

    pub fn record_disconnect(&self, player_name: &str, duration: Duration, transport: TransportKind) {
        counter!(Metric::PlayerDisconnectionsTotal.name(), "transport" => transport.as_str())
            .increment(1);
        histogram!(Metric::SessionDurationSeconds.name(), "transport" => transport.as_str())
            .record(duration.as_secs_f64());
        self.recent_disconnects
            .insert(player_name.to_string(), Instant::now());
        self.emit(TelemetryEvent::PlayerDisconnected {
            at: Utc::now(),
            duration_secs: duration.as_secs(),
        });
    }

    /// A WebSocket connection that reached the listener and was refused before it became a
    /// session — a certificate that verified but named no player, or an upgrade that never
    /// completed. Distinct from a connect failure the client reports: this one is only
    /// visible here.
    pub fn record_websocket_rejection(&self) {
        counter!(Metric::WebsocketHandshakeRejectionsTotal.name()).increment(1);
    }

    // Observable seam for the reconnect window, so the gating logic is testable
    // without reaching into the cache.
    pub fn saw_recent_disconnect(&self, player_name: &str) -> bool {
        self.recent_disconnects.get(player_name).is_some()
    }

    // Emitted only on the graceful shutdown path. A crash cannot emit its own stop
    // event, so a crash is a heartbeat gap with no preceding Server::Stopped.
    pub fn record_stopped(&self) {
        self.emit(TelemetryEvent::Stopped {
            at: Utc::now(),
            uptime_secs: self.started_at.elapsed().as_secs(),
            stop_reason: "graceful",
        });
    }

    /// Whether a Minecraft host could fetch and write a native library, reported by
    /// the Java mod because the mod has no telemetry channel of its own.
    ///
    /// Gated by the same `features.telemetry` flag as everything else here. When it
    /// is off the mod performs no check, so this is never called rather than called
    /// and dropped — but `emit` drops it anyway if the sender is absent.
    pub fn record_host_capability(&self, report: HostCapability) {
        self.emit(TelemetryEvent::ModHostCapability {
            at: Utc::now(),
            report,
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

    // One delivered frame reached one recipient. This is per-recipient, not
    // per-frame: at a fanout of N it runs N times for each serialization the route
    // does, so it is the most frequently executed work on the delivery path. Steady
    // state is two sharded read locks (per-route window plus `any`) and an integer
    // compare; budget accordingly before adding anything here.
    pub fn record_interaction(&self, route: InteractionRoute, sender: u64, recipient: u64) {
        self.interactions.record_delivery(route, sender, recipient);
    }

    pub fn interactions(&self) -> &InteractionTracker {
        &self.interactions
    }

    // A recipient's bounded output queue was full and the frame was dropped for
    // them — the first user-audible routing failure mode under load.
    pub fn record_audio_route_drop(&self) {
        counter!(Metric::AudioRouteRecipientDropsTotal.name()).increment(1);
    }

    // One position datagram put on the wire, with its encoded size. The size
    // histogram is the load-bearing part: it shows headroom against
    // MAX_DATAGRAM_SIZE shrinking as a realm fills, rather than only reporting
    // the failure once packets already exceed it.
    pub fn record_position_datagram(&self, bytes: usize, players: usize) {
        counter!(Metric::PositionDatagramsTotal.name()).increment(1);
        histogram!(Metric::PositionDatagramBytes.name()).record(bytes as f64);
        counter!(Metric::PositionPlayersAdvertisedTotal.name()).increment(players as u64);
    }

    // A position packet could not be encoded within MAX_DATAGRAM_SIZE and was
    // dropped rather than split. Any non-zero rate here means some clients are
    // receiving no position updates at all.
    pub fn record_position_oversize_drop(&self) {
        counter!(Metric::PositionOversizeDropsTotal.name()).increment(1);
    }

    pub fn set_active_players(&self, value: i64) {
        gauge!(Metric::ActivePlayers.name()).set(value as f64);
        self.active_players.store(value, Ordering::Relaxed);
        self.peak_players.fetch_max(value, Ordering::Relaxed);
    }

    pub fn active_players(&self) -> i64 {
        self.active_players.load(Ordering::Relaxed)
    }

    pub fn peak_players(&self) -> i64 {
        self.peak_players.load(Ordering::Relaxed)
    }

    // Drops the high-water mark to the count online right now. A server busy across
    // the UTC day boundary has not gone empty, so resetting to zero would understate
    // the new day until the next connect.
    pub fn reset_peak_players(&self) {
        self.peak_players
            .store(self.active_players.load(Ordering::Relaxed), Ordering::Relaxed);
    }

    pub fn push_peak_players(&self) {
        gauge!(Metric::PeakPlayers.name()).set(self.peak_players.load(Ordering::Relaxed) as f64);
    }

    pub fn set_active_channels(&self, value: i64) {
        gauge!(Metric::ActiveChannels.name()).set(value as f64);
    }

    pub fn set_players_in_channels(&self, value: i64) {
        gauge!(Metric::PlayersInChannels.name()).set(value as f64);
    }

    // 15-minute cadence. Downstream treats a heartbeat inside the last 35 minutes
    // as active, so one missed beat does not flip a live server to inactive.
    pub fn spawn_heartbeat(self: &Arc<Self>, cancel: CancellationToken) -> JoinHandle<()> {
        let service = self.clone();
        tokio::spawn(async move {
            let mut tick = tokio::time::interval(HEARTBEAT_INTERVAL);
            tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            // The first tick of a tokio interval resolves immediately; consuming it
            // here keeps the first heartbeat one full interval after boot instead of
            // duplicating Server::Started.
            tick.tick().await;
            let mut last_utc_date = Utc::now().date_naive();
            loop {
                tokio::select! {
                    _ = tick.tick() => service.emit_heartbeat(&mut last_utc_date),
                    _ = cancel.cancelled() => break,
                }
            }
        })
    }

    // Closes the interaction window and emits the sample. This is the only caller
    // of close_window, which is what makes the window boundary the heartbeat tick.
    pub fn emit_heartbeat(&self, last_utc_date: &mut chrono::NaiveDate) {
        // Read the high-water mark before any reset, so the sample that observes a
        // new UTC day still reports the peak the closing day actually reached in the
        // interval since its last heartbeat.
        let peak_player_count = self.peak_players();
        self.push_peak_players();

        let today = Utc::now().date_naive();
        if today != *last_utc_date {
            self.reset_peak_players();
            *last_utc_date = today;
        }

        let closed = self.interactions.close_window();
        for (route, counts) in closed.iter() {
            gauge!(Metric::PlayersReached.name(), "route" => route.label())
                .set(counts.reached as f64);
            gauge!(Metric::PlayersReachedMutual.name(), "route" => route.label())
                .set(counts.mutual as f64);
        }

        let find = |wanted: InteractionRoute| {
            closed
                .iter()
                .find(|(r, _)| *r == wanted)
                .map(|(_, c)| *c)
                .unwrap_or_default()
        };
        let any = find(InteractionRoute::Any);
        let proximity = find(InteractionRoute::Proximity);
        let channel = find(InteractionRoute::Channel);

        self.emit(TelemetryEvent::Heartbeat {
            at: Utc::now(),
            snapshot: HeartbeatSnapshot {
                uptime_secs: self.started_at.elapsed().as_secs(),
                window_secs: HEARTBEAT_INTERVAL.as_secs(),
                player_count: self.active_players(),
                peak_player_count,
                players_reached: any.reached,
                players_reached_proximity: proximity.reached,
                players_reached_channel: channel.reached,
                players_reached_mutual: any.mutual,
                players_reached_mutual_proximity: proximity.mutual,
                players_reached_mutual_channel: channel.mutual,
                features_enabled: self.features_enabled.clone(),
                recording_enabled: self.recording_enabled,
                recording_active: self.recording_active(),
            },
        });
    }

    pub fn recording_enabled(&self) -> bool {
        self.recording_enabled
    }

    pub fn recording_active(&self) -> bool {
        self.player_state
            .as_ref()
            .is_some_and(|states| states.any_recording())
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
