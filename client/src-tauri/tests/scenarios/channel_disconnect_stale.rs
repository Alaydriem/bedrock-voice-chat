use std::time::Duration;

use bvc_client_lib::testkit::signal::Signal;

use crate::harness::client_proc::ClientProc;
use crate::harness::server::EmbeddedServer;

/// A player who disconnects must not leak their channel membership: a later
/// client reusing the same gamertag, joined to no channel and standing far
/// away, must be governed by proximity alone.
///
/// ## What this proves
///
/// Channel membership is keyed by the cert common name (`game:gamertag`, e.g.
/// `minecraft:Alice`) in both the connection registry's `player_channel` map
/// and the `ChannelCollection`. The disconnect cleanup path
/// (`CacheManager::remove_player`) historically removed entries using the bare
/// gamertag (`Alice`), which never matches the `game:gamertag` key — so a
/// disconnected player's `minecraft:Alice -> channel` mapping survived. When a
/// new connection reused the gamertag, `route_audio_frame` re-derived
/// `minecraft:Alice`, hit the stale entry, and routed the newcomer's audio over
/// the channel bypass — even though that client never joined a channel and was
/// 10 000 blocks away.
///
/// ## Test shape
///
/// Phase 1 (membership live): Alice and Bob both join "leak-test" 10 000 blocks
/// apart. The same-channel bypass routes Alice's audio to Bob despite the gap,
/// proving the `minecraft:Alice -> leak-test` mapping is active.
///
/// Phase 2 (membership must be gone): Alice disconnects gracefully via the
/// production server-switch path (`NetworkStreamManager::reset`), so the server
/// sees a clean CONNECTION_CLOSE and runs its disconnect cleanup immediately
/// rather than waiting out the idle-timeout recovery window. A fresh login code
/// is minted for the same gamertag and a new client connects as "Alice" with NO
/// channel, still 10 000 blocks from Bob. Alice's new audio must be governed by
/// proximity and dropped at the server — Bob receives zero incremental frames
/// from QUIC. `alice2_sent > 0` proves the silence is real (the newcomer's
/// pipeline did produce frames) rather than a dead client.
///
/// Stats are read after each collection window so counters are stable, and
/// Bob's cumulative `frames_from_quic` is baselined across the phase boundary.
///
/// Requires both artifacts to be pre-built:
/// * server cdylib: `cargo build -p bedrock-voice-chat-server` in `server/`
/// * e2e bin: `cargo build -p bedrock-voice-chat-client --bin bvc_client_e2e --features e2e`
#[tokio::test(flavor = "multi_thread")]
async fn disconnect_does_not_leak_channel_membership() {
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

    // Alice creates "leak-test" first; Bob's Connector then finds and joins it,
    // so both share the same channel id.
    let alice = ClientProc::spawn("Alice", &alice_code, &url, "leak-test");
    alice
        .await_connected(Duration::from_secs(30))
        .expect("Alice connects + joins leak-test");

    let bob = ClientProc::spawn("Bob", &bob_code, &url, "leak-test");
    bob
        .await_connected(Duration::from_secs(30))
        .expect("Bob connects + joins leak-test");

    let probe = Signal::musical_probe(48_000);

    // ── PHASE 1: MEMBERSHIP LIVE ─────────────────────────────────────────────
    // 10 000 blocks apart — far beyond every proximity gate. The same-channel
    // bypass must still route Alice's audio to Bob, proving the
    // `minecraft:Alice -> leak-test` mapping exists and is active.
    for _ in 0..5 {
        server.update_positions(&[("Alice", 0.0, 64.0, 0.0), ("Bob", 10_000.0, 64.0, 0.0)]);
        std::thread::sleep(Duration::from_millis(100));
    }
    std::thread::sleep(Duration::from_millis(600));

    let alice_probe_1 = probe.clone();
    let feed_1 = std::thread::spawn(move || {
        alice.feed_tone(&alice_probe_1, 48_000);
        alice
    });

    let cap_1 = bob.collect_captured(Duration::from_millis(3_600));
    let alice = feed_1.join().expect("Phase 1 feed thread panicked");

    let (alice_sent_1, _, _) = alice.stats();
    let (_, bob_from_quic_1, _) = bob.stats();

    let rms_1 = Signal::rms(&Signal::to_mono(&cap_1));

    eprintln!(
        "[channel_disconnect_stale/phase_1] rms={:.6} alice_frames_sent={} \
         bob_frames_from_quic={}",
        rms_1, alice_sent_1, bob_from_quic_1,
    );

    // Sanity: the channel bypass was live for this pair before the disconnect,
    // so Phase 2's silence is attributable to membership cleanup, not a dead
    // pipe.
    assert!(
        alice_sent_1 > 0,
        "Phase 1 FAIL — Alice sent zero AudioFrame datagrams; the input pipeline never \
         produced frames",
    );
    assert!(
        rms_1 > 0.005,
        "Phase 1 FAIL — Bob did not hear Alice in the shared channel at distance 10 000 \
         (rms={rms_1:.6}): the same-channel bypass was not active, so this test cannot \
         prove the leak",
    );

    // ── PHASE 2: MEMBERSHIP MUST BE GONE ─────────────────────────────────────
    // Alice disconnects gracefully (the production server-switch path), so the
    // server sees a clean CONNECTION_CLOSE and runs its disconnect cleanup
    // immediately rather than waiting out the idle-timeout recovery window. The
    // cleanup must drop the `minecraft:Alice -> leak-test` mapping.
    alice
        .disconnect(Duration::from_secs(10))
        .expect("Alice disconnects gracefully");
    alice.shutdown();
    // Brief settle for the server's async disconnect callback to evict the
    // membership before Alice reconnects.
    std::thread::sleep(Duration::from_millis(1_500));

    // A fresh single-use code for the SAME gamertag. The new client joins NO
    // channel (empty channel name -> Connector skips the join), so the only way
    // Bob could hear it is a leaked channel mapping.
    let alice_code_2 = server.login_code("Alice");
    let alice2 = ClientProc::spawn("Alice", &alice_code_2, &url, "");
    alice2
        .await_connected(Duration::from_secs(30))
        .expect("Alice reconnects with no channel");

    for _ in 0..5 {
        server.update_positions(&[("Alice", 0.0, 64.0, 0.0), ("Bob", 10_000.0, 64.0, 0.0)]);
        std::thread::sleep(Duration::from_millis(100));
    }
    std::thread::sleep(Duration::from_millis(600));

    // Drain residual Phase 1 audio and capture Bob's frame baseline AFTER the
    // drain so any in-flight frames are already counted.
    let _ = bob.collect_captured(Duration::from_millis(600));
    let (_, bob_from_quic_baseline, _) = bob.stats();

    let alice2_probe = probe.clone();
    let feed_2 = std::thread::spawn(move || {
        alice2.feed_tone(&alice2_probe, 48_000);
        alice2
    });

    let cap_2 = bob.collect_captured(Duration::from_millis(3_600));
    let alice2 = feed_2.join().expect("Phase 2 feed thread panicked");

    let (alice2_sent, _, _) = alice2.stats();
    let (_, bob_from_quic_cumulative, _) = bob.stats();
    let bob_from_quic_2 = bob_from_quic_cumulative.saturating_sub(bob_from_quic_baseline);

    let rms_2 = Signal::rms(&Signal::to_mono(&cap_2));

    eprintln!(
        "[channel_disconnect_stale/phase_2] rms={:.6} alice2_frames_sent={} \
         bob_frames_from_quic_phase_2={}",
        rms_2, alice2_sent, bob_from_quic_2,
    );

    alice2.shutdown();
    bob.shutdown();

    // The reconnected client did produce frames, so silence at Bob is a real
    // routing decision rather than a stalled pipeline.
    assert!(
        alice2_sent > 0,
        "Phase 2 FAIL — reconnected Alice sent zero AudioFrame datagrams; cannot distinguish \
         a fixed leak from a dead client",
    );

    // The core assertion: the disconnected player's channel membership was
    // cleaned up, so the reconnected gamertag — in no channel, 10 000 blocks
    // away — is governed by proximity and dropped at the server.
    assert_eq!(
        bob_from_quic_2, 0,
        "FAIL — channel membership leaked across disconnect: Bob received {bob_from_quic_2} \
         frames from QUIC from a reconnected Alice who joined no channel and stands 10 000 \
         blocks away. The stale `minecraft:Alice -> leak-test` mapping survived disconnect and \
         routed her over the channel bypass.",
    );

    assert!(
        rms_2 < 0.010,
        "FAIL — Bob is NOT silent (rms={rms_2:.6}, floor=0.010): the reconnected Alice was \
         routed despite never joining a channel; channel membership leaked across disconnect",
    );
}
