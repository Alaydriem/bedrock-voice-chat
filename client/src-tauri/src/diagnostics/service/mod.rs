use std::sync::Arc;
use std::sync::Mutex as StdMutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use common::consts::version::PROTOCOL_VERSION;
use common::structs::audio::NoiseGateStatus;
use common::structs::metrics::{
    LinkDiagnostics, LinkDiagnosticsSnapshot, LinkQuality, LinkSample, MicDiagnostics,
    PeerDiagnostics, PlaybackDiagnostics, SessionDiagnostics,
};
use tokio::sync::watch;

use super::rollup::{RollupBuilder, RollupWindow};
use super::stats::{
    InputPipelineStats, LinkSession, PeerRegistry, QuicLinkStats, SessionConfig, TransportStats,
};
use super::{DeviceInfo, DiagnosticsReport, SampleRing};

mod counter_readings;
mod stall_state;

use counter_readings::CounterReadings;
use stall_state::StallState;

// A live QUIC connection always produces return traffic — acknowledgements above all — so a client
// whose packets are going out while nothing at all comes back is not idle, it is cut off.
//
// This is measured on QUIC packets, not application datagrams. Application datagrams stop whenever
// nobody else is speaking, so basing a stall on them would flag a solo player in an empty channel
// as broken.
//
// Several consecutive ticks are required so one quiet acknowledgement window cannot trip it.
const STALL_TICKS: u32 = 3;

const SNAPSHOT_INTERVAL: Duration = Duration::from_secs(1);
const LOG_INTERVAL: Duration = Duration::from_secs(30);
const ROLLUP_INTERVAL: Duration = Duration::from_secs(300);
// Device names and sample rates change rarely, and reading them contends with the audio
// command path, so they are refreshed on the slow cadence rather than every tick.
const DEVICE_REFRESH_INTERVAL: Duration = Duration::from_secs(30);

pub struct LinkDiagnosticsService {
    quic_stats: watch::Receiver<Arc<QuicLinkStats>>,
    transport: Arc<TransportStats>,
    input: Arc<InputPipelineStats>,
    session: Arc<LinkSession>,
    config: Arc<SessionConfig>,
    peers: Arc<PeerRegistry>,
    devices: Arc<DeviceInfo>,
    ring: StdMutex<SampleRing>,
    last: StdMutex<CounterReadings>,
    stall: StdMutex<StallState>,
    window: StdMutex<RollupWindow>,
    // The most recent ticked snapshot. Reads serve from here rather than recomputing, because
    // building a snapshot advances the delta baseline, the ring, the stall counter and the rollup
    // window — and an on-demand read from a command or a report must not consume a tick's worth of
    // any of those.
    latest: StdMutex<Option<LinkDiagnosticsSnapshot>>,
    // The count of meter messages published to the webview. Reported rather than assumed:
    // without it, a change that halves that traffic and a change that does nothing look
    // identical from outside the process.
    levels: Arc<crate::audio::LevelBus>,
    shutdown: Arc<AtomicBool>,
}

impl LinkDiagnosticsService {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        quic_stats: watch::Receiver<Arc<QuicLinkStats>>,
        transport: Arc<TransportStats>,
        input: Arc<InputPipelineStats>,
        session: Arc<LinkSession>,
        config: Arc<SessionConfig>,
        peers: Arc<PeerRegistry>,
        devices: Arc<DeviceInfo>,
        levels: Arc<crate::audio::LevelBus>,
    ) -> Self {
        Self {
            quic_stats,
            transport,
            input,
            session,
            config,
            peers,
            devices,
            levels,
            ring: StdMutex::new(SampleRing::new()),
            last: StdMutex::new(CounterReadings::default()),
            stall: StdMutex::new(StallState::default()),
            window: StdMutex::new(RollupWindow::default()),
            latest: StdMutex::new(None),
            shutdown: Arc::new(AtomicBool::new(false)),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new_shared(
        quic_stats: watch::Receiver<Arc<QuicLinkStats>>,
        transport: Arc<TransportStats>,
        input: Arc<InputPipelineStats>,
        session: Arc<LinkSession>,
        config: Arc<SessionConfig>,
        peers: Arc<PeerRegistry>,
        devices: Arc<DeviceInfo>,
        levels: Arc<crate::audio::LevelBus>,
    ) -> Arc<Self> {
        Arc::new(Self::new(
            quic_stats,
            transport,
            input,
            session,
            config,
            peers,
            devices,
            levels,
        ))
    }

    pub fn peer_registry(&self) -> Arc<PeerRegistry> {
        self.peers.clone()
    }

    pub fn session_config(&self) -> Arc<SessionConfig> {
        self.config.clone()
    }

    pub fn history(&self) -> Vec<LinkSample> {
        self.ring
            .lock()
            .map(|r| r.samples())
            .unwrap_or_else(|_| Vec::new())
    }

    // Advances one interval and publishes the result. The ticker is the only caller: this is where
    // deltas are consumed, the ring grows, the stall counter moves and the rollup window
    // accumulates.
    //
    // A disconnect clears everything, so a reconnect does not inherit the previous session's
    // trend, stall progress or partial window.
    fn tick(&self) -> Option<LinkDiagnosticsSnapshot> {
        if !self.session.is_connected() {
            self.reset_for_disconnect();
            return None;
        }

        let snapshot = self.build_snapshot(true);
        if let Ok(mut latest) = self.latest.lock() {
            *latest = Some(snapshot.clone());
        }
        Some(snapshot)
    }

    // Advances one interval on demand. Exists because `tick` is driven by a timer the tests cannot
    // wait on without making every case take seconds, and because the distinction between
    // advancing and reading is itself a contract worth testing.
    //
    // Gated so no production caller can reintroduce the advancing-on-read defect this split
    // removed — the service is held in Tauri managed state, so an ungated advancing method would be
    // reachable from any future command.
    #[cfg(any(test, feature = "e2e"))]
    pub fn tick_for_test(&self) -> Option<LinkDiagnosticsSnapshot> {
        self.tick()
    }

    // Read-only. Absent while nothing is connected — a snapshot of zeros renders as a flawless link
    // with a 0 ms round trip, which is worse than no reading at all.
    //
    // Serves the last ticked value with the cheap live fields refreshed, so a panel opening or a
    // report being copied cannot perturb what the next tick measures.
    pub fn snapshot(&self) -> Option<LinkDiagnosticsSnapshot> {
        if !self.session.is_connected() {
            return None;
        }

        let cached = self.latest.lock().ok().and_then(|g| g.clone());
        match cached {
            Some(mut snapshot) => {
                let (input_muted, output_muted) = DeviceInfo::mute_state();
                snapshot.mic.muted = input_muted;
                snapshot.playback.deafened = output_muted;
                snapshot.link.uptime_secs = self.session.uptime_secs();

                // Everything derived from the peer list is recomputed with it. Refreshing the rows
                // while leaving the aggregates at their tick-time values would let one snapshot,
                // and the report printed from it, show a speaker at 40% concealment beside a
                // link-level 0%.
                let peers = self.peers.peers();
                snapshot.link.worst_concealment_pct = peers
                    .iter()
                    .map(|p| p.concealment_pct)
                    .fold(0.0f32, f32::max);
                snapshot.link.jitter_buffer_ms =
                    peers.iter().map(|p| p.buffer_ms).max().unwrap_or(0);
                snapshot.link.jitter_buffer_drops =
                    peers.iter().map(|p| p.overflow_drops + p.ooo_drops).sum();
                snapshot.peers = peers;
                Some(snapshot)
            }
            // Connected but no tick has landed yet. Building one without advancing keeps the very
            // first render honest without stealing the first interval's deltas.
            None => Some(self.build_snapshot(false)),
        }
    }

    // A reconnect must not inherit the previous session's history, stall progress or partial
    // rollup window.
    //
    // The `latest.is_none()` early-out makes this idempotent, and is sound only because `tick` is
    // the sole writer of `latest`. If anything else ever populates it, this would silently skip
    // resetting the ring, the delta baseline and the stall counter.
    fn reset_for_disconnect(&self) {
        if let Ok(mut latest) = self.latest.lock() {
            if latest.is_none() {
                return;
            }
            *latest = None;
        }
        if let Ok(mut ring) = self.ring.lock() {
            ring.clear();
        }
        if let Ok(mut last) = self.last.lock() {
            *last = CounterReadings::default();
        }
        if let Ok(mut stall) = self.stall.lock() {
            *stall = StallState::default();
        }
        self.reset_window();
    }

    // Every cumulative counter this service measures deltas against, read at one instant.
    fn readings_at(&self, now: Instant) -> CounterReadings {
        let quic = self.quic_stats.borrow().clone();
        CounterReadings {
            at: Some(now),
            datagrams_sent: self.transport.datagrams_sent(),
            datagrams_received: self.transport.datagrams_received(),
            audio_frames_sent: self.transport.frames_sent(),
            meter_events: self.levels.emitted(),
            frames_captured: self.input.frames_captured(),
            frames_with_signal: self.input.frames_with_signal(),
            packets_sent: quic.packets_sent(),
            packets_received: quic.packets_received(),
            packets_lost: quic.packets_lost(),
            sequence_received: quic.downlink_loss().map(|(_, r)| r).unwrap_or(0),
            sequence_lost: quic.downlink_loss().map(|(l, _)| l).unwrap_or(0),
            burst_loss: quic.burst_loss(),
        }
    }

    // Restarts every measurement from now, on a link that stays up.
    //
    // Distinct from `reset_for_disconnect`, which zeroes the baseline because the next session's
    // counters will start from zero too. Here the QUIC counters keep climbing, so the baseline is
    // re-read rather than cleared: zeroing it would make the next tick's delta the whole
    // session's traffic and publish one tick of nonsense before settling.
    pub fn reset_stats(&self) {
        let now = Instant::now();
        let current = self.readings_at(now);

        self.peers.reset();
        if let Ok(mut ring) = self.ring.lock() {
            ring.clear();
        }
        if let Ok(mut last) = self.last.lock() {
            *last = current;
        }
        if let Ok(mut stall) = self.stall.lock() {
            *stall = StallState::default();
        }
        self.reset_window();
        // Dropped rather than kept: it holds the peer rows and aggregates that were just
        // zeroed, and serving it would show the old numbers until the next tick lands.
        if let Ok(mut latest) = self.latest.lock() {
            *latest = None;
        }
    }

    fn build_snapshot(&self, advance: bool) -> LinkDiagnosticsSnapshot {
        let quic = self.quic_stats.borrow().clone();
        let now = Instant::now();

        let previous = self
            .last
            .lock()
            .map(|g| g.clone())
            .unwrap_or_default();

        let current = self.readings_at(now);

        let elapsed = previous
            .at
            .map(|t| now.saturating_duration_since(t).as_secs_f32())
            .unwrap_or(0.0);

        let sent_delta = Self::delta(current.datagrams_sent, previous.datagrams_sent);
        let received_delta = Self::delta(current.datagrams_received, previous.datagrams_received);
        let audio_sent_delta = Self::delta(current.audio_frames_sent, previous.audio_frames_sent);
        let captured_delta = Self::delta(current.frames_captured, previous.frames_captured);
        let meter_delta = Self::delta(current.meter_events, previous.meter_events);
        let signal_delta = Self::delta(current.frames_with_signal, previous.frames_with_signal);
        let quic_sent_delta = Self::delta(current.packets_sent, previous.packets_sent);
        let quic_received_delta = Self::delta(current.packets_received, previous.packets_received);
        let quic_lost_delta = Self::delta(current.packets_lost, previous.packets_lost);

        let send_rate = Self::rate(audio_sent_delta, elapsed);
        // Absent rather than zero on the tick that has nothing to diff against. Reported as a
        // measurement it would accuse the capture device of being dead every time a client
        // connects, one tick before the first real reading contradicts it.
        let capture_rate = previous
            .at
            .map(|_| Self::rate(captured_delta, elapsed));
        let recv_rate = Self::rate(received_delta, elapsed);
        let meter_rate = Self::rate(meter_delta, elapsed);
        let uplink_loss_pct = Self::ratio_pct(quic_lost_delta, quic_sent_delta);

        // Downlink from the server's own sequence, over the window rather than cumulatively, so a
        // burst an hour ago does not colour the current reading. Absent while no stamped envelope
        // has arrived.
        let sequence_received_delta =
            Self::delta(current.sequence_received, previous.sequence_received);
        let sequence_lost_delta = Self::delta(current.sequence_lost, previous.sequence_lost);
        let downlink_loss_pct = quic.downlink_loss().map(|_| {
            Self::ratio_pct(
                sequence_lost_delta,
                sequence_received_delta + sequence_lost_delta,
            )
        });

        let burst_delta = Self::delta(current.burst_loss, previous.burst_loss);
        let burst_loss_pct = Self::ratio_pct(burst_delta, received_delta + burst_delta);

        let peers = self.peers.peers();
        let worst_concealment_pct = peers
            .iter()
            .map(|p| p.concealment_pct)
            .fold(0.0f32, f32::max);

        let stalled = self.assess_stall(
            advance,
            previous.at.is_some(),
            quic_sent_delta,
            quic_received_delta,
        );

        let rtt_ms = quic.smoothed_rtt_ms();
        let sample = LinkSample {
            at_ms: Self::now_ms(),
            rtt_ms,
            uplink_loss_pct,
            worst_concealment_pct,
        };

        if advance {
            if let Ok(mut ring) = self.ring.lock() {
                ring.push(sample);
            }
            if let Ok(mut last) = self.last.lock() {
                *last = current;
            }
            if let Ok(mut window) = self.window.lock() {
                window.datagrams_sent += sent_delta;
                window.datagrams_received += received_delta;
                window.packets_sent += quic_sent_delta;
                window.packets_lost += quic_lost_delta;
                window.sequence_received += sequence_received_delta;
                window.sequence_lost += sequence_lost_delta;
                window.sequence_measured |= downlink_loss_pct.is_some();
                window.peer_count = window.peer_count.max(peers.len() as u32);
                window.worst_concealment_pct =
                    window.worst_concealment_pct.max(worst_concealment_pct);
                if stalled {
                    window.stalled_ticks += 1;
                }
            }
        }

        let buffer_ms = peers.iter().map(|p| p.buffer_ms).max().unwrap_or(0);
        let buffer_drops: u64 = peers
            .iter()
            .map(|p| p.overflow_drops + p.ooo_drops)
            .sum();

        // Concealment is not loss and must not be classified as though it were: a quiet speaker
        // conceals heavily and there is nothing wrong with the link. Downlink loss, where measured,
        // does count — it is the direction a listener actually suffers.
        let worst_loss = downlink_loss_pct
            .unwrap_or(burst_loss_pct)
            .max(uplink_loss_pct);
        let quality = LinkQuality::classify(worst_loss, rtt_ms.unwrap_or(0));

        let devices = self.devices.snapshot();
        // Read live rather than from the slow device cache: mute is toggled from a keybind, the UI,
        // an in-game command and a WebSocket client, and a value up to thirty seconds stale would
        // contradict what the user just did.
        let (input_muted, output_muted) = DeviceInfo::mute_state();

        LinkDiagnosticsSnapshot {
            captured_at_ms: sample.at_ms,
            meter_events_per_sec: meter_rate,
            mic: MicDiagnostics {
                device: devices.input_name,
                sample_rate: devices.input_sample_rate,
                noise_gate: NoiseGateStatus::of(
                    DeviceInfo::noise_gate_enabled(),
                    signal_delta > 0,
                ),
                muted: input_muted,
                capture_frames_per_sec: capture_rate,
                datagrams_per_sec: send_rate,
            },
            playback: PlaybackDiagnostics {
                device: devices.output_name,
                sample_rate: devices.output_sample_rate,
                datagrams_per_sec: recv_rate,
                muted_peer_count: devices.muted_peer_count,
                deafened: output_muted,
            },
            link: LinkDiagnostics {
                state: if stalled {
                    "stalled".to_string()
                } else {
                    "connected".to_string()
                },
                uptime_secs: self.session.uptime_secs(),
                rtt_ms,
                rtt_variance_ms: quic.rtt_variance_ms(),
                uplink_loss_pct,
                downlink_loss_pct,
                burst_loss_pct,
                worst_concealment_pct,
                jitter_buffer_ms: buffer_ms,
                jitter_buffer_drops: buffer_drops,
                quic_port: self.session.port(),
                family: self.session.family(),
                paths_used: quic.paths_used(),
                datagrams_dropped: quic.datagrams_dropped(),
                stalled,
                quality,
            },
            session: SessionDiagnostics {
                server: self.session.server(),
                protocol_version: Some(PROTOCOL_VERSION.to_string()),
                proximity_range: self.config.proximity_range(),
                falloff: self.config.falloff(),
                family_preference: devices.family_preference,
                transport: self.session.transport(),
            },
            peers,
            history: self.history(),
        }
    }

    // Sending with nothing coming back, sustained, measured on QUIC packets rather than
    // application datagrams. Silence in both directions is a quiet connection, not a stall, and
    // the very first tick has no previous reading to compare against so it can never be one.
    fn assess_stall(
        &self,
        advance: bool,
        had_previous: bool,
        sent_delta: u64,
        received_delta: u64,
    ) -> bool {
        let mut guard = match self.stall.lock() {
            Ok(g) => g,
            Err(_) => return false,
        };

        if !had_previous {
            return false;
        }

        let starving = sent_delta > 0 && received_delta == 0;
        let consecutive = if starving {
            guard.consecutive.saturating_add(1)
        } else {
            0
        };

        if advance {
            guard.consecutive = consecutive;
        }

        consecutive >= STALL_TICKS
    }

    // A current value below the previous one means the counters were replaced — a reconnect mints
    // a fresh stats handle — so the window yields nothing rather than a negative or enormous rate.
    fn delta(current: u64, previous: u64) -> u64 {
        current.saturating_sub(previous)
    }

    fn rate(delta: u64, elapsed_secs: f32) -> f32 {
        if elapsed_secs <= 0.0 {
            return 0.0;
        }
        delta as f32 / elapsed_secs
    }

    fn ratio_pct(numerator: u64, denominator: u64) -> f32 {
        if denominator == 0 {
            return 0.0;
        }
        ((numerator as f64 / denominator as f64) * 100.0).clamp(0.0, 100.0) as f32
    }

    fn now_ms() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }

    pub fn stop(&self) {
        self.shutdown.store(true, Ordering::SeqCst);
    }

    pub fn start(self: Arc<Self>, app_handle: tauri::AppHandle) {
        let shutdown = self.shutdown.clone();

        tauri::async_runtime::spawn(async move {
            let mut snapshot_tick = tokio::time::interval(SNAPSHOT_INTERVAL);
            let mut log_tick = tokio::time::interval(LOG_INTERVAL);
            let mut rollup_tick = tokio::time::interval(ROLLUP_INTERVAL);
            let mut device_tick = tokio::time::interval(DEVICE_REFRESH_INTERVAL);

            loop {
                if shutdown.load(Ordering::SeqCst) {
                    break;
                }

                tokio::select! {
                    _ = snapshot_tick.tick() => {
                        if let Some(snapshot) = self.tick() {
                            self.publish(&app_handle, snapshot);
                        }
                    }
                    _ = log_tick.tick() => {
                        self.log_report();
                    }
                    _ = rollup_tick.tick() => {
                        self.emit_rollup(&app_handle);
                    }
                    _ = device_tick.tick() => {
                        self.devices.refresh(&app_handle);
                    }
                }
            }
        });
    }

    // Paste-ready text for the copy action and the diagnostics page. One formatter, so a report
    // and a status panel cannot describe the same link differently.
    pub fn render_report(&self) -> String {
        match self.snapshot() {
            Some(snapshot) => DiagnosticsReport::render(&snapshot),
            None => DiagnosticsReport::render_disconnected(),
        }
    }

    // A five minute summary of the client-to-server leg. Carries no peer name, no peer identity
    // and no location; the analytics provider derives this client's own region from the ingest
    // address, which is what makes "does region X have a bad link to server Y" answerable
    // without anything identifying on the wire.
    fn emit_rollup(&self, app_handle: &tauri::AppHandle) {
        let Some(snapshot) = self.snapshot() else {
            self.reset_window();
            return;
        };
        let Some(server_id) = self.session.server_id() else {
            self.reset_window();
            return;
        };

        let window = self
            .window
            .lock()
            .map(|w| w.clone())
            .unwrap_or_default();

        let reportable = self
            .ring
            .lock()
            .map(|ring| RollupBuilder::is_reportable(&ring, &window))
            .unwrap_or(false);

        if !reportable {
            self.reset_window();
            return;
        }

        let peers = &snapshot.peers;
        let mut window = window;
        window.underruns = peers.iter().map(|p| p.underruns).sum();
        window.overflow_drops = peers.iter().map(|p| p.overflow_drops).sum();
        window.ooo_drops = peers.iter().map(|p| p.ooo_drops).sum();
        window.plc_frames = peers.iter().map(|p| p.plc_frames).sum();

        let rollup = match self.ring.lock() {
            Ok(ring) => RollupBuilder::build(
                server_id,
                &snapshot,
                &ring,
                &window,
                self.session.family(),
                self.devices.snapshot().family_preference,
                env!("CARGO_PKG_VERSION").to_string(),
            ),
            Err(_) => {
                self.reset_window();
                return;
            }
        };

        if let Some(analytics) =
            tauri::Manager::try_state::<Arc<crate::analytics::AnalyticsService>>(app_handle)
        {
            analytics.track(
                common::structs::AnalyticsEvent::ClientLinkQuality,
                Some(Self::rollup_properties(&rollup)),
            );
        }

        self.reset_window();
    }

    fn reset_window(&self) {
        if let Ok(mut window) = self.window.lock() {
            *window = RollupWindow::default();
        }
    }

    fn rollup_properties(
        rollup: &common::structs::metrics::LinkRollup,
    ) -> common::structs::AnalyticsEventData {
        let mut data = common::structs::AnalyticsEventData::new()
            .insert("server_id", rollup.server_id.clone())
            .insert("uplink_loss_pct", rollup.uplink_loss_pct)
            .insert("worst_concealment_pct", rollup.worst_concealment_pct)
            .insert("datagrams_sent", rollup.datagrams_sent)
            .insert("datagrams_received", rollup.datagrams_received)
            .insert("underruns", rollup.underruns)
            .insert("overflow_drops", rollup.overflow_drops)
            .insert("ooo_drops", rollup.ooo_drops)
            .insert("plc_frames", rollup.plc_frames)
            .insert("peer_count", rollup.peer_count)
            .insert("samples", rollup.samples)
            .insert("stalled_ticks", rollup.stalled_ticks)
            .insert("protocol_version", rollup.protocol_version.clone())
            .insert("client_version", rollup.client_version.clone());

        // Omitted entirely when unmeasured rather than sent as zero: a server predating the sequence
        // field must not look like a clean link in the fleet view.
        if let Some(v) = rollup.downlink_loss_pct {
            data = data.insert("downlink_loss_pct", v);
        }
        if let Some(v) = rollup.rtt_p50_ms {
            data = data.insert("rtt_p50_ms", v);
        }
        if let Some(v) = rollup.rtt_p95_ms {
            data = data.insert("rtt_p95_ms", v);
        }
        if let Some(v) = rollup.rtt_max_ms {
            data = data.insert("rtt_max_ms", v);
        }
        if let Some(family) = rollup.address_family {
            data = data.insert("address_family", format!("{family:?}"));
        }
        if let Some(preference) = rollup.family_preference {
            data = data.insert("family_preference", format!("{preference:?}"));
        }

        data
    }

    fn publish(&self, app_handle: &tauri::AppHandle, snapshot: LinkDiagnosticsSnapshot) {
        if let Some(broadcaster) =
            tauri::Manager::try_state::<crate::websocket::WebSocketBroadcaster>(app_handle)
        {
            broadcaster.broadcast_metrics(snapshot);
        }
    }

    // The line a support conversation is answered from. Emitted at info so it is present in a
    // log a player can send without being asked to run a different build, and one line per
    // speaker because attributing chop to one voice is the entire point.
    fn log_report(&self) {
        if !self.session.is_connected() {
            return;
        }

        let quic = self.quic_stats.borrow().clone();
        let rtt = quic
            .smoothed_rtt_ms()
            .map(|v| v.to_string())
            .unwrap_or_else(|| "unmeasured".to_string());
        let uplink_loss = Self::ratio_pct(quic.packets_lost(), quic.packets_sent());
        let family = self
            .session
            .family()
            .map(|f| format!("{:?}", f))
            .unwrap_or_else(|| "unknown".to_string());

        for peer in self.peers.peers() {
            Self::log_peer(&peer, &rtt, uplink_loss, &family);
        }
    }

    fn log_peer(peer: &PeerDiagnostics, rtt: &str, uplink_loss: f32, family: &str) {
        log::debug!(
            "Receive diagnostics [{}]: underruns={} overflow_drops={} ooo_drops={} plc={} \
             silence={} decoded={} ring={}/{} warmup={} buffer={}ms quality={:.2} \
             concealment={:.1}% | link rtt={}ms uplink_loss={:.1}% family={}",
            peer.name,
            peer.underruns,
            peer.overflow_drops,
            peer.ooo_drops,
            peer.plc_frames,
            peer.silence_frames,
            peer.frames_decoded,
            peer.ring_len,
            peer.capacity,
            peer.warmup_needed,
            peer.buffer_ms,
            peer.quality_score,
            peer.concealment_pct,
            rtt,
            uplink_loss,
            family,
        );
    }
}
