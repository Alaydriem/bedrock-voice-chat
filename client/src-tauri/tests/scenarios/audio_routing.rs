use std::time::Duration;

use bvc_client_lib::testkit::signal::Signal;

use crate::harness::client_proc::ClientProc;
use crate::harness::server::EmbeddedServer;

/// Two-phase proximity gating test: Alice talks, Bob hears in-range (Phase A)
/// and is silenced out-of-range (Phase B).
///
/// ## What this proves
///
/// BVC has two layers of proximity enforcement:
///   1. **Server-side**: `route_audio_frame` drops packets where
///      `can_communicate_with` fails (distance > 1.73 * broadcast_range ≈ 83
///      blocks with the default `broadcast_range = 48`).
///   2. **Client-side** (fallback): `calculate_spatial_audio_data` zeroes the
///      volume when distance > `falloff_distance = 48`.
///
/// At 10 000 blocks both layers trigger. The test validates the observable
/// outcome (silence at Bob) via server-driven position data.
///
/// ## Channel isolation
///
/// Each client joins its own unique channel ("alice-solo" / "bob-solo") so
/// they are never in the same channel. Since C2 fixed same-channel routing to
/// bypass proximity entirely, placing both clients in the same channel would
/// break the out-of-range-silent assertion. Different channels ensure only
/// the proximity branch governs, which is what this test exercises.
///
/// ## Assertion strategy
///
/// Phase A (in-range): every AudioFrame datagram Alice handed to QUIC arrived
/// at Bob from QUIC, exactly (`bob_from_quic == alice_sent`, 100 % over
/// loopback) AND notes present (rms > 0.005, Goertzel fractions > 0.02). xcorr
/// is logged for diagnostics only.
///
/// Phase B (out-of-range): Bob received ZERO incremental frames from QUIC
/// (`frames_from_quic_phase_b == 0`) AND captured rms < 0.010. The
/// differential guard is direct: Phase A proved the pipe was alive; Phase B
/// proves the server dropped everything at distance 10 000.
///
/// Stats are read after each collection window so counters are stable.
///
/// Requires both artifacts to be pre-built:
/// * server cdylib: `cargo build -p bedrock-voice-chat-server` in `server/`
/// * e2e bin: `cargo build -p bedrock-voice-chat-client --bin bvc_client_e2e --features e2e`
#[tokio::test(flavor = "multi_thread")]
async fn in_range_hears_out_of_range_silent() {
    let data_dir = tempfile::tempdir().expect("create temp data dir");

    let rocket_port = EmbeddedServer::free_port_tcp();
    let quic_port = EmbeddedServer::free_port_udp();

    let config_json = EmbeddedServer::config_json(rocket_port, quic_port, data_dir.path());
    let certs_path = data_dir.path().join("certificates");

    let lib = EmbeddedServer::load_library();
    let server =
        EmbeddedServer::start(lib, &config_json, rocket_port, quic_port, &certs_path).await;

    let alice_code = server.login_code("Alice");
    let bob_code = server.login_code("Bob");

    let url = format!("https://127.0.0.1:{}", server.rocket_port());

    // Each client joins its own isolated channel so the C2 same-channel bypass
    // does not apply — only proximity governs audio routing here.
    let alice = ClientProc::spawn("Alice", &alice_code, &url, "alice-solo");
    alice
        .await_connected(Duration::from_secs(30))
        .expect("Alice connects + joins");

    let bob = ClientProc::spawn("Bob", &bob_code, &url, "bob-solo");
    bob.await_connected(Duration::from_secs(30))
        .expect("Bob connects + joins");

    let probe = Signal::musical_probe(48_000);

    // ── PHASE A: IN RANGE ────────────────────────────────────────────────────
    // Distance 1 block — well within server broadcast_range (48) and client
    // falloff_distance (48). Repeat the position push several times to give the
    // server's QUIC fan-out and the client's moka cache time to settle.
    for _ in 0..5 {
        server.update_positions(&[("Alice", 0.0, 64.0, 0.0), ("Bob", 1.0, 64.0, 0.0)]);
        std::thread::sleep(Duration::from_millis(100));
    }
    std::thread::sleep(Duration::from_millis(500));

    let alice_probe_a = probe.clone();
    let feed_a = std::thread::spawn(move || {
        alice.feed_tone(&alice_probe_a, 48_000);
        alice
    });

    // Collect for 3 600 ms to absorb pipeline latency (~3 s probe + headroom).
    // The 3.6 s window also gives Alice's encoder + QUIC send time to drain.
    let cap_near = bob.collect_captured(Duration::from_millis(3_600));
    let alice = feed_a.join().expect("Phase A feed thread panicked");

    // Read stats after the full collection window so counters are stable.
    let (alice_sent_a, _, _) = alice.stats();
    let (_, bob_from_quic_a, bob_received_a) = bob.stats();

    let mono_near = Signal::to_mono(&cap_near);
    let rms_near = Signal::rms(&mono_near);
    let xcorr_near = Signal::xcorr_peak(&probe, &mono_near);

    let frac_c4 = Signal::tone_energy_fraction(&mono_near, 48_000, 261.63);
    let frac_e4 = Signal::tone_energy_fraction(&mono_near, 48_000, 329.63);
    let frac_g4 = Signal::tone_energy_fraction(&mono_near, 48_000, 392.00);

    eprintln!(
        "[audio_routing/phase_a] captured_samples={} mono_samples={} \
         rms_near={:.6} xcorr_near={:.4} \
         frac_c4={:.4} frac_e4={:.4} frac_g4={:.4} \
         alice_frames_sent={} bob_frames_from_quic={} bob_frames_into_jitter_buffer={}",
        cap_near.len(),
        mono_near.len(),
        rms_near,
        xcorr_near,
        frac_c4,
        frac_e4,
        frac_g4,
        alice_sent_a,
        bob_from_quic_a,
        bob_received_a,
    );

    // Phase A: proximity routing delivered every frame to Bob, exactly. Paced
    // 20 ms delivery keeps the bounded datagram queue from overrunning, so
    // in-range loopback loss is zero.
    assert!(
        alice_sent_a > 0,
        "Phase A FAIL — Alice sent zero AudioFrame datagrams to QUIC: the input pipeline \
         never produced frames",
    );
    assert_eq!(
        bob_from_quic_a, alice_sent_a,
        "Phase A FAIL — QUIC delivery loss at distance 1: Alice sent {alice_sent_a} AudioFrame \
         datagrams but Bob received {bob_from_quic_a} from QUIC (expected exact 100 %); \
         the pipe dropped frames in range",
    );

    assert!(
        rms_near > 0.005,
        "Phase A FAIL — Bob is silent at distance 1 (rms_near={rms_near:.6}): \
         audio never reached Bob; the pipe is broken",
    );

    assert!(
        frac_c4 > 0.02,
        "Phase A FAIL — C4 tone energy missing (frac_c4={frac_c4:.4}): 261.63 Hz did not arrive",
    );
    assert!(
        frac_e4 > 0.02,
        "Phase A FAIL — E4 tone energy missing (frac_e4={frac_e4:.4}): 329.63 Hz did not arrive",
    );
    assert!(
        frac_g4 > 0.02,
        "Phase A FAIL — G4 tone energy missing (frac_g4={frac_g4:.4}): 392.00 Hz did not arrive",
    );

    // ── PHASE B: OUT OF RANGE ────────────────────────────────────────────────
    // Distance 10 000 blocks — far beyond both the server broadcast_range gate
    // (1.73 * 48 ≈ 83 blocks) and the client falloff_distance (48 blocks).
    // The server drops Alice's packets before they reach Bob entirely.
    //
    // Drain Bob's buffer so residual Phase A audio does not bleed into Phase B.
    let _ = bob.collect_captured(Duration::from_millis(500));

    for _ in 0..5 {
        server.update_positions(&[("Alice", 0.0, 64.0, 0.0), ("Bob", 10_000.0, 64.0, 0.0)]);
        std::thread::sleep(Duration::from_millis(100));
    }
    // Extra wait for the position to propagate through QUIC into the client
    // player cache before Alice starts feeding.
    std::thread::sleep(Duration::from_millis(600));

    let alice_probe_b = probe.clone();
    let feed_b = std::thread::spawn(move || {
        alice.feed_tone(&alice_probe_b, 48_000);
        alice
    });

    // 1 600 ms is sufficient to confirm silence — the server drops all of
    // Alice's packets before they reach Bob so there is nothing to decode.
    let cap_far = bob.collect_captured(Duration::from_millis(1_600));
    let alice = feed_b.join().expect("Phase B feed thread panicked");

    // Phase B: read cumulative counters and subtract Phase A baseline.
    let (alice_sent_b, _, _) = alice.stats();
    let (_, bob_from_quic_cumulative, _) = bob.stats();
    let bob_from_quic_b = bob_from_quic_cumulative.saturating_sub(bob_from_quic_a);

    let mono_far = Signal::to_mono(&cap_far);
    let rms_far = Signal::rms(&mono_far);

    eprintln!(
        "[audio_routing/phase_b] captured_samples={} mono_samples={} rms_far={:.6} \
         alice_frames_sent_phase_b={} bob_frames_from_quic_phase_b={}",
        cap_far.len(),
        mono_far.len(),
        rms_far,
        alice_sent_b,
        bob_from_quic_b,
    );

    alice.shutdown();
    bob.shutdown();

    // Phase B gating: the server must have dropped all packets before they
    // reached Bob's QUIC bus. Direct frame accounting: Bob received zero
    // incremental AudioFrame packets from QUIC at distance 10 000.
    assert!(
        bob_from_quic_b == 0,
        "Phase B FAIL — Bob received {bob_from_quic_b} frames from QUIC at distance 10 000 \
         (expected 0): proximity gating did not drop Alice's packets at the server",
    );

    assert!(
        rms_far < 0.010,
        "Phase B FAIL — Bob is NOT silent at distance 10 000 \
         (rms_far={rms_far:.6}, floor=0.010): proximity gating did not silence Bob",
    );
}
