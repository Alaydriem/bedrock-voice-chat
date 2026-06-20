use std::time::Duration;

use bvc_client_lib::testkit::signal::Signal;

use crate::harness::client_proc::ClientProc;
use crate::harness::server::EmbeddedServer;

/// End-to-end audio flow test: Alice emits a musical probe while Bob
/// (co-located in the same channel) captures. Validates the full pipeline:
/// fake-input → noise-gate → Opus encode → QUIC → server fan-out → jitter
/// buffer → decode → spatial mix → fake capture.
///
/// The probe is a C-major 1-3-5-3-1 melody followed by a C-major chord (~3 s),
/// which lets the assertions verify both that delivery was lossless (all frames
/// received from QUIC were forwarded to the jitter buffer) and that the
/// specific note frequencies (C4, E4, G4) survived the Opus decode.
///
/// ## Assertion strategy
///
/// Transport fidelity is the end-to-end gate: every AudioFrame datagram Alice
/// actually handed to QUIC must arrive at Bob from QUIC, exactly:
///   alice frames_sent      — AudioFrame datagrams handed to QUIC by Alice's
///                            network output path (post-encode, real send).
///   bob   frames_from_quic — AudioFrame datagrams Bob received from QUIC.
///
/// With the fake input source pacing frames at a real 20 ms microphone cadence
/// the bounded datagram queue never overruns, so loopback delivery is lossless:
/// `bob_from_quic == alice_sent` (100 %). The "heard experience" is proven
/// separately via Goertzel energy fractions at each triad frequency — these are
/// stable across Opus variance.
///
/// Requires both artifacts to be pre-built:
/// * server cdylib: `cargo build -p bedrock-voice-chat-server` in `server/`
/// * e2e bin: `cargo build -p bedrock-voice-chat-client --bin bvc_client_e2e --features e2e`
#[tokio::test(flavor = "multi_thread")]
async fn two_clients_audio_flows_alice_to_bob() {
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

    // Spawn Alice first and wait for her to connect so she creates the "voice"
    // channel before Bob spawns. Bob's connector then finds the existing channel
    // via list_channels and joins it instead of creating a duplicate.
    let alice = ClientProc::spawn("Alice", &alice_code, &url, "voice");
    alice
        .await_connected(Duration::from_secs(30))
        .expect("Alice connects + joins");

    let bob = ClientProc::spawn("Bob", &bob_code, &url, "voice");
    bob.await_connected(Duration::from_secs(30))
        .expect("Bob connects + joins");

    // Place both players co-located in overworld so can_communicate_with
    // passes (same dimension, distance=0 < proximity threshold).
    for _ in 0..5 {
        server.update_positions(&[("Alice", 0.0, 64.0, 0.0), ("Bob", 0.0, 64.0, 0.0)]);
        std::thread::sleep(Duration::from_millis(100));
    }

    // Give the position caches a moment to propagate before we start audio.
    std::thread::sleep(Duration::from_millis(500));

    let probe = Signal::musical_probe(48_000);

    // Alice enqueues all frames back-to-back; the bin's fake input source paces
    // them out to the DSP and QUIC send at a real 20 ms microphone cadence. Bob
    // collects for 3 600 ms to absorb the ~3 s paced send plus pipeline latency
    // (jitter buffer, Opus decode, spatial mix). Both run concurrently so the
    // capture window overlaps Alice's paced encoding.
    let alice_probe = probe.clone();
    let feed_handle = std::thread::spawn(move || {
        alice.feed_tone(&alice_probe, 48_000);
        alice
    });

    // The 3 600 ms window covers the ~3 s paced send (≈150 frames × 20 ms) plus
    // QUIC + jitter + decode drain before stats are read below.
    let captured = bob.collect_captured(Duration::from_millis(3_600));

    let alice = feed_handle.join().expect("feed thread panicked");

    // Read stats after the full collection window so all counters are stable.
    let (alice_sent, _, _) = alice.stats();
    let (_, bob_from_quic, bob_received) = bob.stats();

    alice.shutdown();
    bob.shutdown();

    let mono = Signal::to_mono(&captured);
    let xcorr = Signal::xcorr_peak(&probe, &mono);
    let rms = Signal::rms(&mono);

    let frac_c4 = Signal::tone_energy_fraction(&mono, 48_000, 261.63);
    let frac_e4 = Signal::tone_energy_fraction(&mono, 48_000, 329.63);
    let frac_g4 = Signal::tone_energy_fraction(&mono, 48_000, 392.00);

    eprintln!(
        "[audio_flow] captured_samples={} mono_samples={} \
         rms={:.6} xcorr={:.4} \
         frac_c4={:.4} frac_e4={:.4} frac_g4={:.4} \
         alice_frames_sent={} bob_frames_from_quic={} bob_frames_into_jitter_buffer={}",
        captured.len(),
        mono.len(),
        rms,
        xcorr,
        frac_c4,
        frac_e4,
        frac_g4,
        alice_sent,
        bob_from_quic,
        bob_received,
    );

    // Transport lossless end-to-end: every AudioFrame datagram Alice handed to
    // QUIC arrived at Bob from QUIC, exactly. Paced 20 ms delivery keeps the
    // bounded datagram queue from overrunning, so loopback loss is zero.
    assert!(
        alice_sent > 0,
        "Alice sent zero AudioFrame datagrams to QUIC — the input pipeline never produced frames",
    );
    assert_eq!(
        bob_from_quic, alice_sent,
        "QUIC delivery loss: Alice sent {alice_sent} AudioFrame datagrams but Bob received \
         {bob_from_quic} from QUIC (expected exact 100 % over loopback)",
    );

    // Some audible signal reached Bob.
    assert!(
        rms > 0.005,
        "Bob's capture is silent (rms={rms:.6}): audio did not reach Bob",
    );

    // Per-note Goertzel fraction: each triad note must carry at least 2 % of
    // total captured power. This proves the specific frequencies survived the
    // Opus decode — a delivery artefact without real content would fail here.
    assert!(
        frac_c4 > 0.02,
        "C4 tone energy too low (frac_c4={frac_c4:.4}): 261.63 Hz did not survive the round-trip",
    );
    assert!(
        frac_e4 > 0.02,
        "E4 tone energy too low (frac_e4={frac_e4:.4}): 329.63 Hz did not survive the round-trip",
    );
    assert!(
        frac_g4 > 0.02,
        "G4 tone energy too low (frac_g4={frac_g4:.4}): 392.00 Hz did not survive the round-trip",
    );
}
