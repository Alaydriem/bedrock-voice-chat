use std::time::Duration;

use bvc_client_lib::testkit::signal::Signal;

use crate::harness::client_proc::ClientProc;
use crate::harness::server::EmbeddedServer;

/// Channel membership governs the same-channel audio bypass through the full
/// explicit lifecycle: join → leave → rejoin → disband.
///
/// ## What this proves
///
/// Channel membership is keyed by the cert common name (`game:gamertag`) in the
/// connection registry's `player_channel` map and the `ChannelCollection`.
/// Every explicit channel operation goes through the real HTTP
/// `api_channel_event` path, which keys by `identity.subject().common_name()`.
/// This test exercises that keying end-to-end with Alice and Bob 10 000 blocks
/// apart (far beyond every proximity gate), so audio only ever reaches Bob when
/// the same-channel bypass is active:
///
/// * Join — both in "lifecycle", Bob hears (bypass active).
/// * Leave — Alice leaves, Bob goes silent (proximity governs again).
/// * Rejoin — Alice rejoins, Bob hears again (bypass restored).
/// * Disband — the channel is deleted, Bob goes silent (membership dropped for
///   both members via `registry.remove_channel`).
///
/// Bob's cumulative `frames_from_quic` is baselined per phase, so each phase's
/// delta isolates exactly what the server routed during that phase. The "silent"
/// phases assert an exact zero incremental delivery — the server dropped every
/// frame before it reached Bob's QUIC bus.
///
/// Requires both artifacts to be pre-built:
/// * server cdylib: `cargo build -p bedrock-voice-chat-server` in `server/`
/// * e2e bin: `cargo build -p bedrock-voice-chat-client --bin bvc_client_e2e --features e2e`
#[tokio::test(flavor = "multi_thread")]
async fn channel_membership_governs_audio_through_join_leave_rejoin_disband() {
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

    let alice = ClientProc::spawn("Alice", &alice_code, &url, "lifecycle");
    alice
        .await_connected(Duration::from_secs(30))
        .expect("Alice connects + joins lifecycle");
    let channel_id = alice
        .await_channel_id(Duration::from_secs(5))
        .expect("Alice reports the joined channel id");

    let bob = ClientProc::spawn("Bob", &bob_code, &url, "lifecycle");
    bob
        .await_connected(Duration::from_secs(30))
        .expect("Bob connects + joins lifecycle");

    let probe = Signal::musical_probe(48_000);

    // Place them 10 000 blocks apart for the whole test, refreshed each phase so
    // the server position cache never goes stale.
    // The trailing settle also covers channel-event propagation: an explicit
    // join/leave/delete is acked when the HTTP call returns, but the server
    // applies it asynchronously when the broadcast ChannelEvent is processed off
    // the webhook loop. Waiting here before feeding ensures membership has fully
    // settled, so a restored bypass routes the whole probe rather than its tail.
    let push_far = || {
        for _ in 0..5 {
            server.update_positions(&[("Alice", 0.0, 64.0, 0.0), ("Bob", 10_000.0, 64.0, 0.0)]);
            std::thread::sleep(Duration::from_millis(100));
        }
        std::thread::sleep(Duration::from_millis(1_500));
    };

    // Feed Alice's probe and report (rms_at_bob, incremental frames Bob pulled
    // from QUIC during this phase). A pre-feed drain clears residual audio and
    // lets the cumulative counter settle before the baseline is taken.
    // Returns (rms_at_bob, frames Bob pulled from QUIC during this phase) and
    // asserts Alice actually emitted the whole probe — so a silent phase proves
    // membership gating dropped real audio, not that the sender went quiet.
    let measure = |alice: &ClientProc, bob: &ClientProc| -> (f32, u64) {
        let _ = bob.collect_captured(Duration::from_millis(400));
        let (a_base, _, _) = alice.stats();
        let (_, base_fq, _) = bob.stats();
        alice.feed_tone(&probe, 48_000);
        let cap = bob.collect_captured(Duration::from_millis(3_600));
        let (a_now, _, _) = alice.stats();
        let (_, fq, _) = bob.stats();
        let alice_sent = a_now.saturating_sub(a_base);
        assert!(
            alice_sent >= 100,
            "harness FAIL — Alice emitted only {alice_sent} frames this phase (expected ~150); \
             the fake input source is not pacing, so a silent result would be meaningless",
        );
        let rms = Signal::rms(&Signal::to_mono(&cap));
        (rms, fq.saturating_sub(base_fq))
    };

    // ── JOIN: both in channel, far apart → Bob hears via the bypass ──────────
    // Membership was established during connect, so by the time audio is fed the
    // bypass is fully active and the whole ~150-frame probe reaches Bob. This is
    // the strong baseline the later "audible" phase is compared against.
    push_far();
    let (rms_join, fq_join) = measure(&alice, &bob);
    eprintln!("[channel_lifecycle/join] rms={rms_join:.6} bob_frames_from_quic={fq_join}");
    assert!(
        fq_join >= 100 && rms_join > 0.005,
        "JOIN FAIL — Bob did not hear the full probe in the shared channel at distance 10 000 \
         (rms={rms_join:.6}, frames_from_quic={fq_join}, expected >= 100): the same-channel \
         bypass was not active",
    );

    // ── LEAVE: Alice leaves → proximity governs → Bob silent ─────────────────
    alice
        .leave_channel(&channel_id, Duration::from_secs(10))
        .expect("Alice leaves the channel");
    push_far();
    let (rms_leave, fq_leave) = measure(&alice, &bob);
    eprintln!("[channel_lifecycle/leave] rms={rms_leave:.6} bob_frames_from_quic={fq_leave}");
    assert_eq!(
        fq_leave, 0,
        "LEAVE FAIL — Bob received {fq_leave} frames from QUIC after Alice left the channel \
         (expected 0): membership was not removed, so the bypass still routed her audio at \
         distance 10 000",
    );
    assert!(
        rms_leave < 0.010,
        "LEAVE FAIL — Bob is NOT silent after Alice left (rms={rms_leave:.6}, floor=0.010)",
    );

    // ── REJOIN: Alice rejoins → bypass restored → Bob hears the full probe ───
    // Membership is re-applied before the paced feed begins, so the whole probe
    // is routed again — the same strong baseline as JOIN.
    alice
        .rejoin_channel(&channel_id, Duration::from_secs(10))
        .expect("Alice rejoins the channel");
    push_far();
    let (rms_rejoin, fq_rejoin) = measure(&alice, &bob);
    eprintln!("[channel_lifecycle/rejoin] rms={rms_rejoin:.6} bob_frames_from_quic={fq_rejoin}");
    assert!(
        fq_rejoin >= 100 && rms_rejoin > 0.005,
        "REJOIN FAIL — Bob did not hear the full probe after Alice rejoined the channel \
         (rms={rms_rejoin:.6}, frames_from_quic={fq_rejoin}, expected >= 100): the bypass was \
         not restored",
    );

    // ── DISBAND: channel deleted → membership dropped for both → Bob silent ──
    alice
        .delete_channel(&channel_id, Duration::from_secs(10))
        .expect("Alice disbands the channel");
    push_far();
    let (rms_disband, fq_disband) = measure(&alice, &bob);
    eprintln!("[channel_lifecycle/disband] rms={rms_disband:.6} bob_frames_from_quic={fq_disband}");

    alice.shutdown();
    bob.shutdown();

    assert_eq!(
        fq_disband, 0,
        "DISBAND FAIL — Bob received {fq_disband} frames from QUIC after the channel was deleted \
         (expected 0): disband did not drop membership for both members",
    );
    assert!(
        rms_disband < 0.010,
        "DISBAND FAIL — Bob is NOT silent after the channel was disbanded \
         (rms={rms_disband:.6}, floor=0.010)",
    );
}
