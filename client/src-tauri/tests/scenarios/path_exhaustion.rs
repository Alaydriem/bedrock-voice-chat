use std::time::Duration;

use bvc_client_lib::testkit::signal::Signal;

use crate::harness::client_proc::ClientProc;
use crate::harness::rebinding_relay::RebindingUdpRelay;
use crate::harness::server::EmbeddedServer;

// Five paths per connection, never reclaimed
// (`s2n-quic-transport::path::manager::MAX_ALLOWED_PATHS`). Rotating past that is
// what stops the server accepting a client's datagrams.
const PATH_BUDGET: u64 = 5;

// Fast enough to spend the budget inside a test window, slow enough that a rotation
// does not land inside the handshake.
const ROTATE_EVERY: Duration = Duration::from_millis(400);

// An idle QUIC connection barely transmits, which would leave nothing to observe.
// Feeding real audio makes the client send continuously at a 20 ms microphone
// cadence: `feed_tone` only enqueues, and the bin paces the frames out, so this many
// seconds of PCM buys that many seconds of upstream traffic.
const AUDIO_SECONDS: f32 = 12.0;

/// Reproduces the reported failure locally, with no NAT64 and no carrier involved.
///
/// A relay forwards the client's QUIC traffic while rebinding its own upstream socket
/// every few hundred milliseconds, which is exactly what a carrier translator pool
/// does to an IPv6-only client forced through CLAT. s2n-quic identifies a path by
/// remote address including port and allows five per connection without ever
/// reclaiming one, so each rotation burns a slot and the sixth is refused with
/// `PathLimitExceeded`.
///
/// The assertion is the return flow. A live QUIC connection produces a steady stream
/// of server-to-client datagrams (ACKs and keep-alive responses); once the budget is
/// spent the server discards the client's datagrams before processing them, so that
/// return flow stops even though the client is still sending. That is the player's
/// symptom — audio degrading and then dying while his packets still reach the server
/// — expressed as something a test can observe.
///
/// This test is what turns "the mechanism exists in the dependency source" into "the
/// mechanism is reproducible here", which is most of what the field capture was for.
///
/// Requires both artifacts to be pre-built:
/// * server cdylib: `cargo build -p bedrock-voice-chat-server` in `server/`
/// * e2e harness: `cargo build -p bvc-client-e2e`
#[tokio::test(flavor = "multi_thread")]
async fn a_rotating_source_address_exhausts_the_path_budget_and_stops_the_return_flow() {
    let data_dir = tempfile::tempdir().expect("create temp data dir");

    let rocket_port = EmbeddedServer::free_port_tcp();
    let quic_port = EmbeddedServer::free_port_udp();
    let relay_port = EmbeddedServer::free_port_udp();

    // The server listens on `quic_port` but advertises the relay's port, so the
    // client dials the relay and every datagram is translated on the way through.
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

    // Real audio, paced out by the bin, is what keeps the client transmitting across
    // the whole rotation window. Without it the connection goes quiet and the
    // server's silence would prove nothing.
    client.feed_tone(&Signal::chirp(48_000, AUDIO_SECONDS, 200.0, 2_000.0), 48_000);
    tokio::time::sleep(Duration::from_millis(500)).await;

    // The connection is healthy on one path: the client is sending and the server is
    // answering. Establishing this first is what makes the later silence meaningful
    // rather than just "nothing ever worked".
    assert!(
        relay.upstream_datagrams() > 0,
        "the relay should have carried client traffic to the server"
    );
    assert!(
        relay.downstream_datagrams() > 0,
        "a healthy connection must produce server-to-client traffic"
    );
    assert_eq!(
        relay.rebinds(),
        0,
        "rotation must not have started before the handshake completed"
    );

    relay.arm();

    // One rotation past the budget: the first five are admitted as paths, the next is
    // refused.
    relay
        .await_rebinds(PATH_BUDGET + 1, Duration::from_secs(20))
        .await
        .expect("relay rotates its source address past the path budget");

    // Let the exhausted state settle, then measure whether anything still comes back.
    tokio::time::sleep(Duration::from_millis(600)).await;
    let downstream_before = relay.downstream_datagrams();
    let upstream_before = relay.upstream_datagrams();

    tokio::time::sleep(Duration::from_secs(2)).await;

    let downstream_after = relay.downstream_datagrams();
    let upstream_after = relay.upstream_datagrams();

    assert!(
        upstream_after > upstream_before,
        "the client must still be sending, or this test proves nothing about the \
         server's behaviour: {upstream_before} -> {upstream_after}"
    );
    assert_eq!(
        downstream_after, downstream_before,
        "the server must have stopped answering once its path budget was spent, but \
         the return flow continued: {downstream_before} -> {downstream_after}. Either \
         the path limit no longer applies (check MAX_ALLOWED_PATHS after an s2n-quic \
         bump) or the relay is not presenting new source addresses."
    );

    relay.stop();
    client.shutdown();
}

/// The control. The same relay, the same traffic, rotation never armed — the
/// connection must stay healthy for the whole window. Without this, the test above
/// would pass just as well if the relay were simply broken.
#[tokio::test(flavor = "multi_thread")]
async fn a_stable_source_address_keeps_the_return_flow_alive() {
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
        .expect("client connects through the relay");

    client.feed_tone(&Signal::chirp(48_000, AUDIO_SECONDS, 200.0, 2_000.0), 48_000);
    tokio::time::sleep(Duration::from_millis(500)).await;

    let downstream_before = relay.downstream_datagrams();

    tokio::time::sleep(Duration::from_secs(2)).await;

    assert_eq!(relay.rebinds(), 0, "rotation was never armed");
    assert!(
        relay.downstream_datagrams() > downstream_before,
        "an un-rotated connection must keep receiving from the server: {} -> {}",
        downstream_before,
        relay.downstream_datagrams()
    );

    relay.stop();
    client.shutdown();
}
