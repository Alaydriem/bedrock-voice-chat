use std::time::{Duration, Instant};

use bvc_client_lib::testkit::signal::Signal;

use crate::harness::client_proc::ClientProc;
use crate::harness::rebinding_relay::RebindingUdpRelay;
use crate::harness::server::EmbeddedServer;

// Matches `path_exhaustion.rs`, which established these values against the real
// `MAX_ALLOWED_PATHS` behaviour.
const PATH_BUDGET: u64 = 5;
const ROTATE_EVERY: Duration = Duration::from_millis(400);
const AUDIO_SECONDS: f32 = 20.0;

// Transmission has to outlast the observation, not the other way round: see the
// top-up loop below for why a single finite tone cannot guarantee that.
const TOP_UP_SECONDS: f32 = 2.0;
const TOP_UP_WINDOW: Duration = Duration::from_secs(2);
const STALL_OBSERVATION: Duration = Duration::from_secs(30);

/// `path_exhaustion.rs` proves the server stops answering once a rotating source address spends
/// its path budget. This proves the *client notices and says so*, which is the difference between
/// a symptom and a diagnostic.
///
/// It also guards the correction this feature is built on: the client cannot detect this with a
/// `PathLimitExceeded` counter, because the budget is spent on the server and from here the
/// server's address never changes — one path, no drops. The only client-visible signature is the
/// one asserted below, sending while nothing returns.
///
/// Requires both artifacts pre-built:
/// * server cdylib: `cargo build -p bedrock-voice-chat-server` in `server/`
/// * e2e harness: `cargo build -p bvc-client-e2e`
#[tokio::test(flavor = "multi_thread")]
async fn a_stalled_return_flow_is_reported_as_stalled_in_the_snapshot() {
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

    let relay = RebindingUdpRelay::start(relay_port, server.quic_port(), ROTATE_EVERY).await;

    let code = server.login_code("Alice");
    let url = format!("https://127.0.0.1:{}", server.rocket_port());
    let client = ClientProc::spawn("Alice", &code, &url, "test-channel");

    client
        .await_connected(Duration::from_secs(20))
        .expect("client connects through the relay on a stable path");

    // Real audio, paced by the bin, is what keeps the client transmitting across the rotation
    // window. Without it the connection goes quiet and silence would prove nothing.
    client.feed_tone(
        &Signal::chirp(48_000, AUDIO_SECONDS, 200.0, 2_000.0),
        48_000,
    );

    // Healthy first, so the later stall is a change rather than a state that never worked.
    let healthy = client
        .await_diagnostics(
            |(connected, stalled, _)| *connected && !*stalled,
            Duration::from_secs(15),
        )
        .expect("a healthy connection must report connected and not stalled");
    assert!(!healthy.1, "not stalled while the relay is still stable");

    relay.arm();
    relay
        .await_rebinds(PATH_BUDGET + 1, Duration::from_secs(20))
        .await
        .expect("relay rotates its source address past the path budget");

    // The derivation sets `stalled` only after STALL_TICKS consecutive ticks of
    // sending with nothing returning, so the client must STILL be transmitting when
    // the window is read. A single finite tone cannot promise that: everything
    // before this point — connect, the healthy wait, the rotations — spends from
    // the same budget, and once the tone drains, `sent` is zero, the consecutive
    // counter resets, and no stall can ever be reported however dead the return
    // path is. Topping the feed up leaves whether the client NOTICES as the only
    // variable, which is the thing under test.
    let deadline = Instant::now() + STALL_OBSERVATION;
    let mut last = client.diagnostics();
    let stalled = loop {
        client.feed_tone(
            &Signal::chirp(48_000, TOP_UP_SECONDS, 200.0, 2_000.0),
            48_000,
        );
        match client.await_diagnostics(|(_, stalled, _)| *stalled, TOP_UP_WINDOW) {
            Ok(reading) => break reading,
            Err(_) => last = client.diagnostics(),
        }
        assert!(
            Instant::now() < deadline,
            "the client must report a stall once the server stops answering. Last reading \
             (connected, stalled, uptime)={last:?}; relay rebinds={}, downstream datagrams={}, \
             client frames sent={}. Read those first: traffic still returning means the path \
             limit no longer applies (check MAX_ALLOWED_PATHS after an s2n-quic bump), while a \
             dead return flow that never set this means the stall derivation's consecutive-tick \
             threshold no longer matches the snapshot cadence.",
            relay.rebinds(),
            relay.downstream_datagrams(),
            client.stats().0,
        );
    };

    assert!(stalled.1, "stalled must be set");
    assert!(
        stalled.0,
        "the link must still report as connected while stalled — a stall that looked like a \
         disconnect would hide exactly the case this exists to explain"
    );

    client.shutdown();
}

/// The guard against a derivation that trips on ordinary jitter. A false stall in the field is
/// worse than none: it would send players chasing a network problem that does not exist.
#[tokio::test(flavor = "multi_thread")]
async fn a_healthy_connection_never_reports_stalled() {
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

    // Same relay, never armed: the traffic still crosses it, but the source address holds still.
    let relay = RebindingUdpRelay::start(relay_port, server.quic_port(), ROTATE_EVERY).await;

    let code = server.login_code("Bob");
    let url = format!("https://127.0.0.1:{}", server.rocket_port());
    let client = ClientProc::spawn("Bob", &code, &url, "test-channel");

    client
        .await_connected(Duration::from_secs(20))
        .expect("client connects");

    client.feed_tone(&Signal::chirp(48_000, 10.0, 200.0, 2_000.0), 48_000);

    client
        .await_diagnostics(|(connected, _, _)| *connected, Duration::from_secs(15))
        .expect("diagnostics become available once connected");

    // Sample across several stall windows. Three consecutive send-with-no-return ticks would set
    // it, so a healthy link has to outlast that.
    //
    // `uptime_secs` is the positive control. Without it every assertion here passes when the ticker
    // never ran at all: reads serve a cache that stays empty, and the uncached path reports
    // `stalled: false` unconditionally because it has no previous reading to compare against. A
    // dead ticker would make this test green while proving nothing.
    let mut saw_a_tick = false;
    for _ in 0..8 {
        let (connected, stalled, uptime) = client.diagnostics();
        assert!(connected, "the link must stay connected");
        assert!(!stalled, "a healthy connection must never report a stall");
        saw_a_tick |= uptime > 0;
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    assert!(
        saw_a_tick,
        "no tick ever landed, so the absence of a stall proves nothing"
    );

    assert_eq!(relay.rebinds(), 0, "the relay must not have rotated");

    client.shutdown();
}
