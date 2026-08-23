use std::time::Duration;

use bvc_client_lib::testkit::signal::Signal;

use crate::harness::client_proc::ClientProc;
use crate::harness::server::EmbeddedServer;

/// Same-channel members hear each other regardless of distance.
///
/// ## What this proves
///
/// C2 added a channel-membership fast path to `route_audio_frame`: when the
/// sender and receiver share a channel, the server routes audio regardless of
/// the spatial distance between them. This test places Alice and Bob 10 000
/// blocks apart — well beyond both the server `broadcast_range` gate
/// (1.73 * 48 ≈ 83 blocks) and the client `falloff_distance` (48 blocks) —
/// yet Alice's audio must reach Bob because they are in the same channel.
///
/// This is the inverse of `audio_routing::in_range_hears_out_of_range_silent`,
/// which proves that players in *different* channels are still governed by
/// proximity.
///
/// ## Assertion strategy
///
/// Transport fidelity: every AudioFrame datagram Alice handed to QUIC arrived
/// at Bob from QUIC, exactly (`bob_from_quic == alice_sent`, 100 % over
/// loopback — the same-channel bypass routes all of them despite the 10 000
/// block gap). Heard experience: rms > 0.005 and per-note Goertzel energy
/// fractions for C4/E4/G4 each exceed 2 %. xcorr is logged for diagnostics but
/// is not a pass gate.
///
/// Stats are read after the full 3 600 ms collection window so both counters
/// are stable (all frames encoded + routed + ingested).
///
/// Requires both artifacts to be pre-built:
/// * server cdylib: `cargo build -p bedrock-voice-chat-server` in `server/`
/// * e2e harness: `cargo build -p bvc-client-e2e`
#[tokio::test(flavor = "multi_thread")]
async fn same_channel_members_hear_regardless_of_distance() {
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

    // Spawn Alice first and wait for her to connect so she creates "shared-voice"
    // before Bob spawns. Bob's Connector then finds the channel via list_channels
    // and joins it — both clients share the same channel id.
    let alice = ClientProc::spawn("Alice", &alice_code, &url, "shared-voice");
    alice
        .await_connected(Duration::from_secs(30))
        .expect("Alice connects + joins shared-voice");

    let bob = ClientProc::spawn("Bob", &bob_code, &url, "shared-voice");
    bob.await_connected(Duration::from_secs(30))
        .expect("Bob connects + joins shared-voice");

    // Place them 10 000 blocks apart — far beyond every proximity gate —
    // then wait for the position to propagate through the server's QUIC fan-out
    // and the clients' moka position caches.
    for _ in 0..5 {
        server.update_positions(&[("Alice", 0.0, 64.0, 0.0), ("Bob", 10_000.0, 64.0, 0.0)]);
        std::thread::sleep(Duration::from_millis(100));
    }
    std::thread::sleep(Duration::from_millis(600));

    let probe = Signal::musical_probe(48_000);

    // Alice enqueues all frames back-to-back; the bin's fake input source paces
    // them out to QUIC at a real 20 ms microphone cadence.
    let alice_probe = probe.clone();
    let feed_handle = std::thread::spawn(move || {
        alice.feed_tone(&alice_probe, 48_000);
        alice
    });

    // The 3 600 ms window covers the ~3 s paced send (≈150 frames × 20 ms) plus
    // QUIC + jitter + decode drain before stats are read below.
    let captured = bob.collect_captured(Duration::from_millis(3_600));

    let alice = feed_handle.join().expect("feed thread panicked");

    // Read stats after the full collection window so counters are stable.
    let (alice_sent, _, _) = alice.stats();
    let (_, bob_from_quic, bob_received) =
        bob.await_transport_frames(alice_sent, Duration::from_secs(5));

    alice.shutdown();
    bob.shutdown();

    let mono = Signal::to_mono(&captured);
    let rms = Signal::rms(&mono);
    let xcorr = Signal::xcorr_peak(&probe, &mono);

    let frac_c4 = Signal::tone_energy_fraction(&mono, 48_000, 261.63);
    let frac_e4 = Signal::tone_energy_fraction(&mono, 48_000, 329.63);
    let frac_g4 = Signal::tone_energy_fraction(&mono, 48_000, 392.00);

    eprintln!(
        "[same_channel] captured_samples={} mono_samples={} \
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

    // Channel-membership bypass is active and lossless: every AudioFrame
    // datagram Alice handed to QUIC arrived at Bob from QUIC, exactly, despite
    // the 10 000 block gap. Paced 20 ms delivery keeps the bounded datagram
    // queue from overrunning over loopback.
    assert!(
        alice_sent > 0,
        "FAIL — Alice sent zero AudioFrame datagrams to QUIC: the input pipeline never \
         produced frames",
    );
    assert_eq!(
        bob_from_quic, alice_sent,
        "FAIL — QUIC delivery loss in the same channel: Alice sent {alice_sent} AudioFrame \
         datagrams but Bob received {bob_from_quic} from QUIC (expected exact 100 %); \
         check whether route_audio_frame logs IN_CHANNEL for this pair",
    );

    // Some audible signal reached Bob despite the 10 000 block gap.
    assert!(
        rms > 0.005,
        "FAIL — Bob is silent at distance 10 000 in the same channel \
         (rms={rms:.6}): the channel-membership bypass did not route audio",
    );

    assert!(
        frac_c4 > 0.02,
        "FAIL — C4 tone energy missing (frac_c4={frac_c4:.4}): 261.63 Hz did not arrive",
    );
    assert!(
        frac_e4 > 0.02,
        "FAIL — E4 tone energy missing (frac_e4={frac_e4:.4}): 329.63 Hz did not arrive",
    );
    assert!(
        frac_g4 > 0.02,
        "FAIL — G4 tone energy missing (frac_g4={frac_g4:.4}): 392.00 Hz did not arrive",
    );
}
