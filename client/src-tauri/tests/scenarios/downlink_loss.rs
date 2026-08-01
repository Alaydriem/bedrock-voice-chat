use std::time::Duration;

use bvc_client_lib::testkit::signal::Signal;

use crate::harness::client_proc::ClientProc;
use crate::harness::lossy_relay::LossyUdpRelay;
use crate::harness::server::EmbeddedServer;

// One downstream datagram in ten discarded. Realistic for a bad path, and gentle enough that QUIC's
// own recovery keeps the connection alive — at far higher rates the handshake and control traffic
// suffer too and the test would be measuring connection collapse rather than loss reporting.
const DROP_ONE_IN: u64 = 10;

const AUDIO_SECONDS: f32 = 20.0;

// Measured on loopback: a 1-in-10 induced downstream drop is reported by the client as ~8% downlink
// loss, against 0% on the same topology unarmed. The band below is set around that.
//
// The client reports whatever share of the server's sequence never arrived. That is not the same
// number as the relay's UDP drop rate: QUIC may coalesce several application datagrams into one UDP
// packet, so discarding one packet can cost more than one sequence number. The assertion is therefore
// a band around the induced rate rather than an equality — tight enough to catch a derivation that is
// wrong by an order of magnitude or reporting a constant, loose enough not to encode an assumption
// about QUIC's packetisation.
const MIN_EXPECTED_LOSS_PCT: f32 = 2.0;
const MAX_EXPECTED_LOSS_PCT: f32 = 60.0;

/// The end-to-end proof for the whole downlink-loss chain: the server stamps a per-connection
/// sequence, the network discards some of it, and the client derives its own loss rate from the gaps
/// without being told anything.
///
/// This is the test that distinguishes a working derivation from a plausible one. The unit tests feed
/// sequences in directly; only this exercises the real stamp at the real fan-out, through a real
/// QUIC connection, with real loss.
///
/// Requires both artifacts pre-built:
/// * server cdylib: `cargo build -p bedrock-voice-chat-server` in `server/`
/// * e2e harness: `cargo build -p bvc-client-e2e`
#[tokio::test(flavor = "multi_thread")]
async fn a_lossy_downlink_is_reported_as_downlink_loss() {
    let data_dir = tempfile::tempdir().expect("create temp data dir");

    let rocket_port = EmbeddedServer::free_port_tcp();
    let quic_port = EmbeddedServer::free_port_udp();
    let relay_port = EmbeddedServer::free_port_udp();

    // The server listens on `quic_port` but advertises the relay's port, so the client dials the
    // relay and every datagram passes through something that can discard it.
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

    let relay = LossyUdpRelay::start(relay_port, server.quic_port(), DROP_ONE_IN).await;

    let url = format!("https://127.0.0.1:{}", server.rocket_port());

    // Two clients, because the measurement needs dense downstream traffic. Alice speaks; Bob is the
    // one under observation, and what he receives is Alice's audio at a 20 ms cadence — roughly fifty
    // stamped envelopes a second, which is enough for a 1-in-10 drop to show up as sequence gaps
    // within a window. A lone client's downstream is mostly acknowledgements and would barely move.
    let alice_code = server.login_code("Alice");
    let alice = ClientProc::spawn("Alice", &alice_code, &url, "test-channel");
    alice
        .await_connected(Duration::from_secs(20))
        .expect("Alice connects through the relay");

    let bob_code = server.login_code("Bob");
    let client = ClientProc::spawn("Bob", &bob_code, &url, "test-channel");
    client
        .await_connected(Duration::from_secs(20))
        .expect("Bob connects through the relay before any loss is induced");

    alice.feed_tone(&Signal::chirp(48_000, AUDIO_SECONDS, 200.0, 2_000.0), 48_000);

    // Bob must actually be hearing Alice before loss is induced, or the sparse-traffic failure mode
    // would masquerade as a broken derivation.
    client
        .await_diagnostics(
            |(connected, _, uptime)| *connected && *uptime > 0,
            Duration::from_secs(20),
        )
        .expect("Bob's diagnostics become available");

    // Clean first. This is the positive control for the whole mechanism: a measured figure here
    // proves the server stamped, the client read the stamp, and the derivation ran. Without it, a
    // later non-zero reading could equally be a bug.
    let clean = client
        .await_diagnostics_downlink_loss(
            |loss| loss.is_some(),
            Duration::from_secs(20),
        )
        .expect(
            "downlink loss must be measured on a clean link. `None` here means the server never \
             stamped a sequence or the client never read one — check that the fan-out in \
             connection_registry stamps and that the envelope field survives the datagram round trip.",
        );

    assert!(
        clean < MIN_EXPECTED_LOSS_PCT,
        "a clean link must report near-zero downlink loss, got {clean}"
    );

    relay.arm();

    let lossy = client
        .await_diagnostics_downlink_loss(
            |loss| loss.is_some_and(|v| v >= MIN_EXPECTED_LOSS_PCT),
            Duration::from_secs(30),
        )
        .expect(
            "induced downlink loss must be reported. If the clean assertion above passed and this \
             fails, the derivation is reporting a constant rather than measuring gaps.",
        );

    assert!(
        (MIN_EXPECTED_LOSS_PCT..=MAX_EXPECTED_LOSS_PCT).contains(&lossy),
        "reported loss {lossy}% should sit near the induced 1-in-{DROP_ONE_IN} rate"
    );

    assert!(
        relay.dropped() > 0,
        "the relay must actually have discarded datagrams, or this proves nothing"
    );
    assert!(
        relay.forwarded() > 0,
        "the relay must still be forwarding, or the connection died rather than degraded"
    );

    client.shutdown();
    alice.shutdown();
}

/// The guard against a derivation that manufactures loss. A false reading in the field is worse than
/// none: it would send an operator chasing a network problem that does not exist.
#[tokio::test(flavor = "multi_thread")]
async fn a_clean_downlink_reports_no_loss() {
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

    // Same relay, never armed: the traffic still crosses it, so the topology is identical and the
    // only difference is whether anything is discarded.
    let relay = LossyUdpRelay::start(relay_port, server.quic_port(), DROP_ONE_IN).await;

    let url = format!("https://127.0.0.1:{}", server.rocket_port());

    let alice_code = server.login_code("Alice");
    let alice = ClientProc::spawn("Alice", &alice_code, &url, "test-channel");
    alice
        .await_connected(Duration::from_secs(20))
        .expect("Alice connects");

    let bob_code = server.login_code("Bob");
    let client = ClientProc::spawn("Bob", &bob_code, &url, "test-channel");
    client
        .await_connected(Duration::from_secs(20))
        .expect("Bob connects");

    alice.feed_tone(&Signal::chirp(48_000, 14.0, 200.0, 2_000.0), 48_000);

    let measured = client
        .await_diagnostics_downlink_loss(|loss| loss.is_some(), Duration::from_secs(20))
        .expect("downlink loss must be measured");

    // Sample repeatedly: a single clean reading could be the first window before anything was
    // observed, whereas a derivation that drifts upward would show it here.
    let mut readings = vec![measured];
    for _ in 0..6 {
        tokio::time::sleep(Duration::from_millis(500)).await;
        if let Some(Some(loss)) = {
            let _ = client.diagnostics();
            client.diagnostic_downlink_loss()
        } {
            readings.push(loss);
        }
    }

    for loss in &readings {
        assert!(
            *loss < MIN_EXPECTED_LOSS_PCT,
            "a clean link must never report meaningful loss; readings were {readings:?}"
        );
    }

    assert_eq!(relay.dropped(), 0, "the relay must not have discarded anything");
    assert!(relay.forwarded() > 0, "traffic must have crossed the relay");

    client.shutdown();
    alice.shutdown();
}
