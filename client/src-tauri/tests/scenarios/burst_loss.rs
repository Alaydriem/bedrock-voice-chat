use std::time::Duration;

use bvc_client_lib::testkit::signal::Signal;

use crate::harness::client_proc::ClientProc;
use crate::harness::drop_pattern::DropPattern;
use crate::harness::lossy_relay::LossyUdpRelay;
use crate::harness::server::EmbeddedServer;

// The per-peer table keys on the display name, which drops the game prefix
// (`Game::display_name`, sink_manager/mod.rs:307); the existing peer_diagnostics scenario asserts
// on the same form.
const SPEAKER: &str = "Alice";

// Alice's audio arrives at Bob at roughly fifty datagrams a second, so a window of fifty is about
// one second, and two consecutive drops per window is one short burst per second. QUIC may pack
// more than one 20 ms frame into a datagram, so each burst costs Bob two to four consecutive frames.
const BURST_EVERY: u64 = 50;
const BURST_CONSECUTIVE: u64 = 2;

const AUDIO_SECONDS: f32 = 20.0;
const LOSSY_WINDOW: Duration = Duration::from_secs(15);

// With forward-accept, a consecutive loss costs exactly the frames that were lost and nothing after
// them, so out-of-order rejections over the window stay near the reordering floor. The unfixed
// acceptance band rejects roughly the next 48 frames after every burst that opens a gap wider than
// 40 ms — around 700 over this window. The bound sits well inside that ratio.
const MAX_OOO_DROPS: u64 = 30;

// About 750 frames are due in the window; ~15 bursts of up to 4 frames removes at most ~60. The
// unfixed tree decodes a fraction of the remainder because each burst also discards the frames
// that follow it.
const MIN_FRAMES_DECODED: u64 = 500;

/// A short consecutive loss must cost the listener only the frames that were lost.
///
/// On the buffer's old acceptance rule, each burst discarded roughly the next second of intact
/// frames: 376 rejections and 347 frames decoded over this window, against 0 and 722 now.
///
/// Requires both artifacts pre-built:
/// * server cdylib: `mise run cdylib`
/// * e2e harness: `mise run e2e-bin`
#[tokio::test(flavor = "multi_thread")]
async fn a_consecutive_loss_costs_only_the_lost_frames() {
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

    let relay = LossyUdpRelay::start_with(
        relay_port,
        server.quic_port(),
        DropPattern::Burst {
            every: BURST_EVERY,
            consecutive: BURST_CONSECUTIVE,
        },
    )
    .await;

    let url = format!("https://127.0.0.1:{}", server.rocket_port());

    let alice_code = server.login_code("Alice");
    let alice = ClientProc::spawn("Alice", &alice_code, &url, "test-channel");
    alice
        .await_connected(Duration::from_secs(20))
        .expect("Alice connects through the relay");

    let bob_code = server.login_code("Bob");
    let bob = ClientProc::spawn("Bob", &bob_code, &url, "test-channel");
    bob.await_connected(Duration::from_secs(20))
        .expect("Bob connects through the relay before any loss is induced");

    alice.feed_tone(
        &Signal::chirp(48_000, AUDIO_SECONDS, 200.0, 2_000.0),
        48_000,
    );

    // Positive control: Bob is decoding Alice on a clean link before anything is discarded.
    let clean = bob
        .await_peer_stat(SPEAKER, |s| s.frames_decoded >= 50, Duration::from_secs(20))
        .expect(
            "Bob decodes Alice on a clean link; if this fails the topology is wrong, not the buffer",
        );

    relay.arm();
    tokio::time::sleep(LOSSY_WINDOW).await;

    let lossy = bob
        .await_peer_stat(SPEAKER, |_| true, Duration::from_secs(5))
        .expect("a reading after the lossy window");

    let ooo = lossy.ooo_drops - clean.ooo_drops;
    let decoded = lossy.frames_decoded - clean.frames_decoded;

    assert!(
        relay.dropped() > 0,
        "the relay must actually have discarded datagrams, or this proves nothing"
    );
    assert!(
        relay.forwarded() > 0,
        "the relay must still be forwarding, or the connection died rather than degraded"
    );
    assert!(
        ooo <= MAX_OOO_DROPS,
        "out-of-order rejections over the window: {ooo} (max {MAX_OOO_DROPS}); \
         a figure in the hundreds is the acceptance band rejecting the frames after each burst. \
         relay dropped={} forwarded={} clean={clean:?} lossy={lossy:?}",
        relay.dropped(),
        relay.forwarded()
    );
    assert!(
        decoded >= MIN_FRAMES_DECODED,
        "frames decoded over the window: {decoded} (min {MIN_FRAMES_DECODED}); \
         relay dropped={} forwarded={} clean={clean:?} lossy={lossy:?}",
        relay.dropped(),
        relay.forwarded()
    );

    alice.shutdown();
    bob.shutdown();
}
