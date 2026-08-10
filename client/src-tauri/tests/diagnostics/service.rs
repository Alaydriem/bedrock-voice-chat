use common::structs::metrics::TransportKind;
use std::sync::Arc;
use std::time::Duration;

use bvc_client_lib::diagnostics::{
    DeviceInfo, InputPipelineStats, LinkDiagnosticsService, LinkSession, PeerRegistry, QuicLinkStats,
    SessionConfig, TransportStats,
};
use bvc_client_lib::audio::LevelBus;
use common::structs::audio::NoiseGateStatus;
use common::structs::reachability::AddressFamily;
use tokio::sync::watch;

// The service deliberately holds no AppHandle. Fields carrying one drag the whole GUI stack into
// a test binary through drop glue, so the handle is a parameter of `start` instead — which is
// also what makes every one of these tests possible.
struct Harness {
    service: LinkDiagnosticsService,
    transport: Arc<TransportStats>,
    input: Arc<InputPipelineStats>,
    session: Arc<LinkSession>,
    quic_tx: watch::Sender<Arc<QuicLinkStats>>,
    quic: Arc<QuicLinkStats>,
    levels: Arc<LevelBus>,
}

impl Harness {
    fn new() -> Self {
        let quic = Arc::new(QuicLinkStats::new());
        let (quic_tx, quic_rx) = watch::channel(quic.clone());
        let transport = Arc::new(TransportStats::new());
        let input = Arc::new(InputPipelineStats::new());
        let session = Arc::new(LinkSession::new());
        let levels = LevelBus::new_shared();

        let service = LinkDiagnosticsService::new(
            quic_rx,
            transport.clone(),
            input.clone(),
            session.clone(),
            Arc::new(SessionConfig::new()),
            PeerRegistry::new_shared(),
            Arc::new(DeviceInfo::new()),
            levels.clone(),
        );

        Self {
            service,
            transport,
            input,
            session,
            quic_tx,
            quic,
            levels,
        }
    }

    fn connect(&self) {
        self.session
            .set(Some(AddressFamily::Ipv4), 443, TransportKind::Quic, "bvc.example.com".to_string(), "ca");
    }

    // Advances one interval, the way the real ticker does. `snapshot()` is read-only, so tests
    // that need state to move must go through this.
    fn tick(&self) -> Option<common::structs::metrics::LinkDiagnosticsSnapshot> {
        self.service.tick_for_test()
    }

    // The first tick has no previous reading, so it establishes the baseline all deltas are
    // measured against.
    fn baseline(&self) {
        let _ = self.tick();
    }
}

#[test]
fn no_snapshot_without_a_connection() {
    let h = Harness::new();

    // Zeros would render as a flawless link with a 0 ms round trip, which misleads worse than an
    // empty panel.
    assert!(h.service.snapshot().is_none());
}

#[test]
fn a_snapshot_is_produced_once_connected() {
    let h = Harness::new();
    h.connect();

    let snapshot = h.tick().expect("connected");
    assert_eq!(snapshot.link.quic_port, Some(443));
    assert_eq!(snapshot.link.family, Some(AddressFamily::Ipv4));
}

#[test]
fn rate_is_derived_from_monotonic_counters_over_an_interval() {
    let h = Harness::new();
    h.connect();
    h.baseline();

    let sends = 50u64;
    for _ in 0..sends {
        h.transport.record_sent();
        h.transport.record_frame_sent();
    }
    let elapsed = Duration::from_millis(200);
    std::thread::sleep(elapsed);

    let rate = h.tick().expect("connected").mic.datagrams_per_sec;

    // A range, not just `> 0`. Asserting positivity alone passes against an implementation that
    // returns the raw delta and never divides by the interval at all, which is precisely what this
    // test is named for. 50 sends over ~200 ms is ~250/s; allow generous scheduling slack but
    // exclude both the undivided delta (50) and an order-of-magnitude error.
    assert!(
        (100.0..1_000.0).contains(&rate),
        "expected a per-second rate near 250, got {rate}"
    );
}

#[test]
fn a_counter_reset_between_ticks_yields_zero_not_a_negative_rate() {
    let h = Harness::new();
    h.connect();

    for _ in 0..100 {
        h.transport.record_sent();
        h.transport.record_frame_sent();
        h.quic.record_sent();
        h.quic.record_lost();
    }
    h.baseline();

    // A reconnect mints a fresh stats handle, so the current reading is below the previous one.
    h.quic_tx
        .send(Arc::new(QuicLinkStats::new()))
        .expect("publish a fresh handle");
    std::thread::sleep(Duration::from_millis(50));

    let snapshot = h.tick().expect("connected");

    // A non-saturating subtraction would panic in a debug build before reaching this.
    assert_eq!(snapshot.link.uplink_loss_pct, 0.0);
    // The rate must be exactly zero, not merely non-negative: an unsigned delta can never be
    // negative, so `>= 0.0` asserts nothing about the reset having been handled.
    assert_eq!(
        snapshot.mic.datagrams_per_sec, 0.0,
        "a window with no new sends must report no rate"
    );
}

/// The row is headed "Your mic", and it was fed from every datagram this client sends.
/// Position, presence, control and health traffic all leave over the same socket and keep that
/// number in the dozens on their own, so it read as a working microphone on a client whose
/// capture stream was dead — and sent a real report of a dead microphone the wrong way.
#[test]
fn the_mic_send_rate_counts_audio_frames_not_every_datagram_this_client_sends() {
    let h = Harness::new();
    h.connect();
    h.baseline();

    for _ in 0..200 {
        h.transport.record_sent();
    }
    std::thread::sleep(Duration::from_millis(50));

    assert_eq!(
        h.tick().expect("connected").mic.datagrams_per_sec,
        0.0,
        "non-audio traffic must not appear as a microphone sending"
    );
}

/// Capture is counted off the device, upstream of the gate, the encoder and the network. It is
/// the only reading that separates a microphone that stopped from audio that stopped getting
/// through, and nothing read it before: the counter existed and no diagnostic consulted it.
#[test]
fn capture_is_reported_from_frames_off_the_device() {
    let h = Harness::new();
    h.connect();
    h.baseline();

    for _ in 0..25 {
        h.input.record_frame(true);
    }
    std::thread::sleep(Duration::from_millis(100));

    let rate = h
        .tick()
        .expect("connected")
        .mic
        .capture_frames_per_sec
        .expect("a measured interval reports a rate");

    // A range rather than positivity: asserting `> 0` passes against an implementation that
    // reports the raw delta and never divides by the interval. 25 frames over ~100 ms is
    // ~250/s.
    assert!(
        (100.0..1_000.0).contains(&rate),
        "expected a per-second rate near 250, got {rate}"
    );
}

/// The figure the whole meter path is tuned against. Two fixed-rate emitters spent about twenty
/// webview messages a second on the meters; on Android each one is a unit of main-thread work on
/// the thread that paints them. Reported rather than assumed, because a change that halves that
/// traffic and a change that does nothing look identical from outside the process.
#[test]
fn meter_messages_are_counted_so_the_traffic_can_be_seen() {
    let h = Harness::new();
    h.connect();
    h.baseline();

    for _ in 0..5 {
        h.levels.record_emitted();
    }
    std::thread::sleep(Duration::from_millis(100));

    let rate = h.tick().expect("connected").meter_events_per_sec;

    // A range rather than positivity: `> 0` passes against an implementation that reports the
    // raw delta and never divides by the interval. 5 messages over ~100 ms is ~50/s.
    assert!(
        (20.0..200.0).contains(&rate),
        "expected a per-second rate near 50, got {rate}"
    );
}

/// Zero here accuses the capture device, and the tick that has nothing to diff against would
/// make that accusation on every connect — one tick before the first real reading contradicts
/// it.
#[test]
fn capture_is_unmeasured_rather_than_zero_before_an_interval_has_passed() {
    let h = Harness::new();
    h.connect();

    assert_eq!(
        h.tick().expect("connected").mic.capture_frames_per_sec,
        None
    );
}

/// The signature of the fault this exists for: the device stops delivering while the client's
/// uplink keeps moving, because position and presence traffic is unaffected by a dead
/// microphone. Both readings have to be visible independently or the panel cannot show it.
#[test]
fn a_dead_capture_device_reads_as_zero_while_other_traffic_continues() {
    let h = Harness::new();
    h.connect();
    h.input.record_frame(false);
    h.baseline();

    for _ in 0..100 {
        h.transport.record_sent();
    }
    std::thread::sleep(Duration::from_millis(50));

    let snapshot = h.tick().expect("connected");
    assert_eq!(snapshot.mic.capture_frames_per_sec, Some(0.0));
    assert_eq!(snapshot.mic.datagrams_per_sec, 0.0);
}

#[test]
fn uplink_loss_is_the_ratio_of_lost_to_sent_in_the_window() {
    let h = Harness::new();
    h.connect();
    h.baseline();

    for _ in 0..100 {
        h.quic.record_sent();
    }
    for _ in 0..5 {
        h.quic.record_lost();
    }

    let snapshot = h.tick().expect("connected");
    assert!(
        (snapshot.link.uplink_loss_pct - 5.0).abs() < 0.01,
        "expected 5% uplink loss, got {}",
        snapshot.link.uplink_loss_pct
    );
}

#[test]
fn gate_is_open_when_frames_with_signal_advanced_in_the_window() {
    let h = Harness::new();
    h.connect();
    h.baseline();
    DeviceInfo::set_noise_gate_enabled(true);

    h.input.record_frame(false);

    assert_eq!(
        h.tick().expect("connected").mic.noise_gate,
        NoiseGateStatus::Open
    );
}

#[test]
fn gate_is_closed_when_no_frame_carried_signal() {
    let h = Harness::new();
    h.connect();
    h.baseline();
    DeviceInfo::set_noise_gate_enabled(true);

    // Frames are still arriving; none of them carry signal.
    for _ in 0..10 {
        h.input.record_frame(true);
    }

    assert_eq!(
        h.tick().expect("connected").mic.noise_gate,
        NoiseGateStatus::Closed
    );
}

/// Silence with the gate switched off is silence, not a gate holding the mic shut. The
/// readout reported `open`/`closed` from signal alone, so it accused a gate that was not
/// even in the audio path — and said `open` when it was not attached at all.
#[test]
fn a_disabled_gate_is_reported_as_disabled_whatever_the_signal_did() {
    let h = Harness::new();
    h.connect();
    h.baseline();
    DeviceInfo::set_noise_gate_enabled(false);

    for _ in 0..10 {
        h.input.record_frame(true);
    }
    assert_eq!(
        h.tick().expect("connected").mic.noise_gate,
        NoiseGateStatus::Disabled
    );

    h.input.record_frame(false);
    assert_eq!(
        h.tick().expect("connected").mic.noise_gate,
        NoiseGateStatus::Disabled
    );
}

#[test]
fn sending_with_nothing_returning_across_the_threshold_sets_stalled() {
    let h = Harness::new();
    h.connect();
    h.baseline();

    // A live QUIC connection always answers with acknowledgements, so QUIC packets leaving while
    // none arrive is not a quiet link — it is a peer that has stopped processing this connection.
    let mut stalled = false;
    for _ in 0..5 {
        h.quic.record_sent();
        stalled = h.tick().expect("connected").link.stalled;
    }

    assert!(stalled, "sustained send-with-no-return must report stalled");
}

#[test]
fn a_single_quiet_tick_does_not_set_stalled() {
    let h = Harness::new();
    h.connect();
    h.baseline();

    h.quic.record_sent();

    // One quiet acknowledgement window is not a stall.
    assert!(!h.tick().expect("connected").link.stalled);
}

#[test]
fn application_datagram_silence_alone_is_never_a_stall() {
    let h = Harness::new();
    h.connect();
    h.baseline();

    // A solo speaker in an empty channel sends audio and receives no application datagrams at
    // all, while QUIC keeps acknowledging underneath. Measuring the stall on application
    // datagrams would have called that broken.
    for _ in 0..10 {
        h.transport.record_sent();
        h.quic.record_sent();
        h.quic.record_received();
        assert!(!h.tick().expect("connected").link.stalled);
    }
}

#[test]
fn return_traffic_clears_a_developing_stall() {
    let h = Harness::new();
    h.connect();
    h.baseline();

    for _ in 0..2 {
        h.quic.record_sent();
        let _ = h.tick();
    }

    h.quic.record_sent();
    h.quic.record_received();
    assert!(!h.tick().expect("connected").link.stalled);

    // ...and the counter restarted rather than resuming where it left off.
    h.quic.record_sent();
    assert!(!h.tick().expect("connected").link.stalled);
}

#[test]
fn silence_in_both_directions_is_not_a_stall() {
    let h = Harness::new();
    h.connect();
    h.baseline();

    // Nothing sent, nothing received: a quiet connection, not a broken one.
    for _ in 0..10 {
        assert!(!h.tick().expect("connected").link.stalled);
    }
}

#[test]
fn mute_state_reflects_the_live_flags_rather_than_a_default() {
    let h = Harness::new();
    h.connect();

    // These two fields were reported as a hardcoded `false` at one point, which is exactly the
    // class of defect this whole feature exists to eliminate: a field that looks like a
    // measurement and is actually a placeholder. The assertion is that the snapshot tracks the
    // live flags, in both directions.
    let (want_muted, want_deafened) = bvc_client_lib::diagnostics::DeviceInfo::mute_state();

    let snapshot = h.tick().expect("connected");
    assert_eq!(snapshot.mic.muted, want_muted);
    assert_eq!(snapshot.playback.deafened, want_deafened);
}

#[test]
fn reading_a_snapshot_does_not_advance_the_ring() {
    let h = Harness::new();
    h.connect();

    // A panel opening, a report being copied, and a command polling all call `snapshot()`. Building
    // one consumes a delta interval, so if reads advanced state they would steal measurements from
    // the ticker and inflate every rate.
    for _ in 0..10 {
        let _ = h.service.snapshot();
        let _ = h.service.render_report();
    }

    assert!(
        h.service.history().is_empty(),
        "reads must not grow the history, got {}",
        h.service.history().len()
    );

    // The ring is only half the guard. The actual rate-inflation vector was a read overwriting the
    // delta baseline, so a later tick measured a fraction of its true interval. After the reads
    // above, the very first tick must still see no previous reading and therefore report no rate.
    h.transport.record_frame_sent();
    let first = h.tick().expect("connected");
    assert_eq!(
        first.mic.datagrams_per_sec, 0.0,
        "reads must not have established a delta baseline"
    );
}

#[test]
fn an_interleaved_read_does_not_suppress_a_real_stall() {
    let h = Harness::new();
    h.connect();
    h.tick();

    // A command polling between ticks must not reset the consecutive-stall counter, or a UI that
    // happens to be open would hide the very condition it exists to display.
    let mut stalled = false;
    for _ in 0..5 {
        h.quic.record_sent();
        let _ = h.service.snapshot();
        stalled = h.tick().expect("connected").link.stalled;
        let _ = h.service.snapshot();
    }

    assert!(stalled, "an interleaved read must not suppress a stall");
}

#[test]
fn a_disconnect_clears_the_history_so_a_reconnect_starts_clean() {
    let h = Harness::new();
    h.connect();
    for _ in 0..5 {
        h.tick();
    }
    assert_eq!(h.service.history().len(), 5);

    h.session.clear();
    h.tick();

    // Carrying the old session's trend forward would show a reconnected client a graph of a
    // connection that no longer exists.
    assert!(h.service.history().is_empty());
    assert!(h.service.snapshot().is_none());

    h.connect();
    h.tick();
    assert_eq!(h.service.history().len(), 1);
}

#[test]
fn history_accumulates_across_ticks() {
    let h = Harness::new();
    h.connect();

    for _ in 0..5 {
        h.tick();
    }

    assert_eq!(h.service.history().len(), 5);
}

#[test]
fn a_report_is_rendered_in_both_connected_and_disconnected_states() {
    let h = Harness::new();

    let disconnected = h.service.render_report();
    assert!(
        disconnected.contains("Not connected"),
        "a disconnected report must say so rather than print zeros: {disconnected}"
    );

    h.connect();
    let connected = h.service.render_report();
    assert!(connected.contains("Link"));
    assert!(connected.contains("Per speaker"));
}
