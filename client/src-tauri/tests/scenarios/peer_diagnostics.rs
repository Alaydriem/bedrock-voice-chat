use std::time::Duration;

use bvc_client_lib::testkit::signal::Signal;

use crate::harness::client_proc::ClientProc;
use crate::harness::server::EmbeddedServer;

/// The evidence behind #232's acceptance criterion.
///
/// The support log line is built from the per-speaker diagnostics table, so the criterion — "a
/// choppy-audio report is attributable to arrival or emission from the listener's existing log" —
/// depends on that table being populated, keyed by a name a human recognises, from a real session.
/// Every other test drives the counters directly; this one proves they are actually reached and
/// attributed when audio crosses two real clients through a real server.
///
/// Alice speaks, Bob listens. Bob's diagnostics must name Alice.
///
/// Requires both artifacts to be pre-built:
/// * server cdylib: `cargo build -p bedrock-voice-chat-server` in `server/`
/// * e2e harness: `cargo build -p bvc-client-e2e`
#[tokio::test(flavor = "multi_thread")]
async fn a_listener_attributes_received_audio_to_the_speaker_by_name() {
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

    // Alice first, so she creates the channel Bob then joins.
    let alice = ClientProc::spawn("Alice", &alice_code, &url, "voice");
    alice
        .await_connected(Duration::from_secs(20))
        .expect("Alice connects");

    let bob = ClientProc::spawn("Bob", &bob_code, &url, "voice");
    bob.await_connected(Duration::from_secs(20))
        .expect("Bob connects");

    // Real audio through the real pipeline: encode, QUIC, server fan-out, jitter buffer.
    alice.feed_tone(&Signal::chirp(48_000, 6.0, 200.0, 2_000.0), 48_000);

    let peers = {
        let deadline = std::time::Instant::now() + Duration::from_secs(20);
        loop {
            let _ = bob.diagnostics();
            let peers = bob.diagnostic_peers();
            if !peers.is_empty() {
                break peers;
            }
            if std::time::Instant::now() >= deadline {
                break Vec::new();
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    };

    assert!(
        peers.iter().any(|name| name == "Alice"),
        "Bob's per-speaker diagnostics must name Alice, or the support log line has nothing to \
         attribute chop to. Got: {peers:?}"
    );

    // Alice hears nobody, so her table must not name Bob.
    //
    // That assertion is worthless on its own: `diagnostics()` leaves the peer list untouched on a
    // timeout, so an empty list would also satisfy it if the bridge were dead or the snapshot were
    // absent. The positive control is that Alice's own reading arrived and reports a live
    // connection, which is what makes the absence of Bob meaningful.
    let (alice_connected, _, alice_uptime) = alice
        .await_diagnostics(
            |(connected, _, uptime)| *connected && *uptime > 0,
            Duration::from_secs(20),
        )
        .expect("Alice's own diagnostics must arrive before absence proves anything");
    assert!(alice_connected);
    assert!(alice_uptime > 0);

    let alice_peers = alice.diagnostic_peers();
    assert!(
        !alice_peers.iter().any(|name| name == "Bob"),
        "Bob never spoke, so he must not appear in Alice's table. Got: {alice_peers:?}"
    );

    bob.shutdown();
    alice.shutdown();
}
