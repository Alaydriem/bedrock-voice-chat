use std::time::{Duration, Instant};

use bvc_client_lib::testkit::PeerStat;
use bvc_client_lib::testkit::signal::Signal;

use crate::harness::client_proc::ClientProc;
use crate::harness::drop_pattern::DropPattern;
use crate::harness::lossy_relay::LossyUdpRelay;
use crate::harness::server::EmbeddedServer;

const SPEAKER: &str = "Alice";

const NEAR: f32 = 24.0;
const FAR: f32 = 48.0;

// One full approach and retreat. Twenty seconds of audio covers two and a half of them, so the
// separation crosses the whole band five times in each direction.
const CYCLE: Duration = Duration::from_secs(8);

// The rate a game mod pushes positions at.
const TICK: Duration = Duration::from_millis(250);

const AUDIO_SECONDS: f32 = 20.0;
const TICKS: u32 = 80;

// About 1 000 frames are due; anything far below that means the listener stopped decoding, which
// the delivery assertions alone would not show.
const MIN_FRAMES_DECODED: u64 = 800;

/// Separation between the two players at `elapsed`: a triangle wave from `NEAR` out to `FAR` and
/// back, so the players approach and retreat without pause.
struct Separation;

impl Separation {
    fn at(elapsed: Duration) -> f32 {
        let phase = (elapsed.as_secs_f32() / CYCLE.as_secs_f32()).fract();
        let ramp = if phase < 0.5 { phase * 2.0 } else { (1.0 - phase) * 2.0 };
        NEAR + (FAR - NEAR) * ramp
    }

    // Both players move: each sits half the separation from the origin, on opposite sides, with a
    // sideways drift so the bearing changes as well as the distance.
    fn push(server: &EmbeddedServer, elapsed: Duration) -> f32 {
        let d = Self::at(elapsed);
        let drift = (elapsed.as_secs_f32() * 0.7).sin() * 4.0;
        server.update_positions(&[
            ("Alice", -d / 2.0, 64.0, drift),
            ("Bob", d / 2.0, 64.0, drift),
        ]);
        d
    }
}

/// Two players in proximity, moving toward and away from each other between 24 and 48 blocks,
/// must lose no audio frames anywhere between the speaker's encoder and the listener's decoder.
///
/// The band sits inside the server's proximity gate (`1.73 * broadcast_range`, 83 blocks at the
/// default 48), so every frame must be delivered. The listener's volume curve is not flat across
/// the band: it is 0 dB at 24 blocks, about -20 dB at 36, and 0 at 48. The captured level per tick
/// is printed against distance so a quiet voice is not mistaken for a lost one.
///
/// Each client joins its own channel so that only the proximity branch routes audio.
///
/// Requires both artifacts pre-built:
/// * server cdylib: `mise run cdylib`
/// * e2e harness: `mise run e2e-bin`
#[tokio::test(flavor = "multi_thread")]
async fn players_moving_between_24_and_48_blocks_lose_no_frames() {
    let data_dir = tempfile::tempdir().expect("create temp data dir");

    let rocket_port = EmbeddedServer::free_port_tcp();
    let quic_port = EmbeddedServer::free_port_udp();

    let config_json = EmbeddedServer::config_json(rocket_port, quic_port, data_dir.path());
    let certs_path = data_dir.path().join("certificates");

    let lib = EmbeddedServer::load_library();
    let server =
        EmbeddedServer::start(lib, &config_json, rocket_port, quic_port, &certs_path).await;

    let url = format!("https://127.0.0.1:{}", server.rocket_port());

    let alice_code = server.login_code("Alice");
    let alice = ClientProc::spawn("Alice", &alice_code, &url, "alice-solo");
    alice
        .await_connected(Duration::from_secs(30))
        .expect("Alice connects + joins");

    let bob_code = server.login_code("Bob");
    let bob = ClientProc::spawn("Bob", &bob_code, &url, "bob-solo");
    bob.await_connected(Duration::from_secs(30))
        .expect("Bob connects + joins");

    // Settle both players at the near edge before any audio, so the first frame already has a
    // position on both sides.
    for _ in 0..5 {
        Separation::push(&server, Duration::ZERO);
        std::thread::sleep(Duration::from_millis(100));
    }
    std::thread::sleep(Duration::from_millis(500));
    let _ = bob.drain_captured();

    let probe = Signal::chirp(48_000, AUDIO_SECONDS, 200.0, 2_000.0);
    let feed = std::thread::spawn(move || {
        alice.feed_tone(&probe, 48_000);
        alice
    });

    let start = Instant::now();
    let mut profile: Vec<(f32, f32)> = Vec::with_capacity(TICKS as usize);
    for _ in 0..TICKS {
        let d = Separation::push(&server, start.elapsed());
        let captured = bob.collect_captured(TICK);
        profile.push((d, Signal::rms(&Signal::to_mono(&captured))));
    }

    let alice = feed.join().expect("feed thread panicked");

    let (alice_sent, _, _) = alice.stats();
    let (_, bob_from_quic, _) = bob.await_transport_frames(alice_sent, Duration::from_secs(10));
    let (_, _, bob_into_buffer) =
        bob.await_jitter_buffer_frames(bob_from_quic, Duration::from_secs(5));
    let peer = bob
        .await_peer_stat(SPEAKER, |_| true, Duration::from_secs(5))
        .expect("Bob has a per-peer reading for Alice");

    for (tick, (d, rms)) in profile.iter().enumerate() {
        eprintln!("[proximity_movement] tick={tick:02} distance={d:5.1} rms={rms:.5}");
    }
    let summary = format!(
        "alice_sent={alice_sent} bob_from_quic={bob_from_quic} bob_into_buffer={bob_into_buffer} \
         peer={peer:?}"
    );
    eprintln!("[proximity_movement] {summary}");

    assert!(alice_sent > 0, "Alice sent no frames; {summary}");
    assert_eq!(
        bob_from_quic, alice_sent,
        "frames lost between Alice and Bob's transport (server proximity skip or delivery loss); \
         {summary}"
    );
    assert_eq!(
        bob_into_buffer, bob_from_quic,
        "Bob's router discarded frames it took off the transport; {summary}"
    );
    assert_eq!(
        peer.spatial_gap_frames, 0,
        "Bob's jitter buffer saw timestamp gaps on the spatial route; {summary}"
    );
    assert_eq!(
        peer.ooo_drops, 0,
        "Bob's jitter buffer rejected frames; {summary}"
    );
    assert!(
        peer.frames_decoded >= MIN_FRAMES_DECODED,
        "Bob decoded {} frames (min {MIN_FRAMES_DECODED}); {summary}",
        peer.frames_decoded
    );

    alice.shutdown();
    bob.shutdown();
}

// Frames the buffer cannot see as a gap: a loss after the last frame that arrives has no later
// frame to open the gap. One burst's worth, allowing for QUIC packing frames into a datagram.
const TAIL_SLACK: u64 = 8;

// The reordering floor. The buffer rejected none on the group route under the same loss.
const MAX_OOO_DROPS: u64 = 5;

/// What one moving-proximity run under loss cost the listener.
struct LossOutcome {
    relay_dropped: u64,
    alice_sent: u64,
    bob_from_quic: u64,
    bob_into_buffer: u64,
    peer: PeerStat,
}

impl LossOutcome {
    fn lost(&self) -> u64 {
        self.alice_sent.saturating_sub(self.bob_from_quic)
    }

    fn summary(&self) -> String {
        let lost = self.lost();
        let percent = 100.0 * lost as f64 / self.alice_sent.max(1) as f64;
        format!(
            "relay_dropped={} alice_sent={} bob_from_quic={} lost={lost} ({percent:.1}%) \
             bob_into_buffer={} peer={:?}",
            self.relay_dropped, self.alice_sent, self.bob_from_quic, self.bob_into_buffer, self.peer
        )
    }

    /// The loss must land on the spatial route, cost only the frames that were lost, and not
    /// stop the listener decoding.
    fn assert_costs_only_what_was_lost(&self) {
        let summary = self.summary();
        let lost = self.lost();
        eprintln!("[proximity_movement/lossy] {summary}");

        assert!(self.relay_dropped > 0, "the relay discarded nothing; {summary}");
        assert!(lost > 0, "no audio frame was lost, so this measured nothing; {summary}");
        assert_eq!(
            self.bob_into_buffer, self.bob_from_quic,
            "Bob's router discarded frames it took off the transport; {summary}"
        );
        assert_eq!(
            self.peer.normal_gap_frames, 0,
            "proximity loss was credited to the group route; {summary}"
        );
        assert!(
            self.peer.spatial_gap_frames <= lost,
            "the buffer counted more gap frames than the transport lost; {summary}"
        );
        assert!(
            self.peer.spatial_gap_frames + TAIL_SLACK >= lost,
            "frames lost in transport are missing from the spatial gap count; {summary}"
        );
        assert!(
            self.peer.ooo_drops <= MAX_OOO_DROPS,
            "the buffer rejected intact frames after a loss; {summary}"
        );
        assert!(
            self.peer.frames_decoded + lost + TAIL_SLACK >= self.alice_sent,
            "Bob decoded fewer frames than arrived; {summary}"
        );
    }
}

/// Runs the moving-proximity scenario with every client datagram routed through a lossy relay.
struct LossyProximityRun;

impl LossyProximityRun {
    async fn measure(pattern: DropPattern) -> LossOutcome {
        let data_dir = tempfile::tempdir().expect("create temp data dir");

        let rocket_port = EmbeddedServer::free_port_tcp();
        let quic_port = EmbeddedServer::free_port_udp();
        let relay_port = EmbeddedServer::free_port_udp();

        let config_json = EmbeddedServer::config_json_advertising(
            rocket_port,
            quic_port,
            data_dir.path(),
            &[relay_port],
        );
        let certs_path = data_dir.path().join("certificates");

        let lib = EmbeddedServer::load_library();
        let server =
            EmbeddedServer::start(lib, &config_json, rocket_port, quic_port, &certs_path).await;
        let relay = LossyUdpRelay::start_with(relay_port, server.quic_port(), pattern).await;

        let url = format!("https://127.0.0.1:{}", server.rocket_port());

        let alice_code = server.login_code("Alice");
        let alice = ClientProc::spawn("Alice", &alice_code, &url, "alice-solo");
        alice
            .await_connected(Duration::from_secs(30))
            .expect("Alice connects through the relay");

        let bob_code = server.login_code("Bob");
        let bob = ClientProc::spawn("Bob", &bob_code, &url, "bob-solo");
        bob.await_connected(Duration::from_secs(30))
            .expect("Bob connects through the relay");

        for _ in 0..5 {
            Separation::push(&server, Duration::ZERO);
            std::thread::sleep(Duration::from_millis(100));
        }
        std::thread::sleep(Duration::from_millis(500));

        // Loss starts with the audio, so every frame Alice sends is exposed to it.
        relay.arm();

        let probe = Signal::chirp(48_000, AUDIO_SECONDS, 200.0, 2_000.0);
        let feed = std::thread::spawn(move || {
            alice.feed_tone(&probe, 48_000);
            alice
        });

        let start = Instant::now();
        for _ in 0..TICKS {
            Separation::push(&server, start.elapsed());
            std::thread::sleep(TICK);
        }

        let alice = feed.join().expect("feed thread panicked");
        let (alice_sent, _, _) = alice.stats();
        let (_, bob_from_quic, _) = bob.await_transport_frames(alice_sent, Duration::from_secs(5));
        let (_, _, bob_into_buffer) =
            bob.await_jitter_buffer_frames(bob_from_quic, Duration::from_secs(5));
        let peer = bob
            .await_peer_stat(SPEAKER, |_| true, Duration::from_secs(5))
            .expect("Bob has a per-peer reading for Alice");

        let outcome = LossOutcome {
            relay_dropped: relay.dropped(),
            alice_sent,
            bob_from_quic,
            bob_into_buffer,
            peer,
        };

        alice.shutdown();
        bob.shutdown();
        outcome
    }
}

/// The same movement as the clean run, with one short burst of two datagrams lost every fifty:
/// about 4% loss, the shape a congested uplink produces.
///
/// Requires both artifacts pre-built:
/// * server cdylib: `mise run cdylib`
/// * e2e harness: `mise run e2e-bin`
#[tokio::test(flavor = "multi_thread")]
async fn moving_players_under_short_bursts_lose_only_the_lost_frames() {
    LossyProximityRun::measure(DropPattern::Burst {
        every: 50,
        consecutive: 2,
    })
    .await
    .assert_costs_only_what_was_lost();
}

/// The same movement under heavy loss: four consecutive datagrams of every twenty, 20%.
///
/// Requires both artifacts pre-built:
/// * server cdylib: `mise run cdylib`
/// * e2e harness: `mise run e2e-bin`
#[tokio::test(flavor = "multi_thread")]
async fn moving_players_under_heavy_bursts_lose_only_the_lost_frames() {
    LossyProximityRun::measure(DropPattern::Burst {
        every: 20,
        consecutive: 4,
    })
    .await
    .assert_costs_only_what_was_lost();
}
