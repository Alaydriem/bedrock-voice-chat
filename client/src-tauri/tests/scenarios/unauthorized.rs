use std::time::Duration;

use crate::harness::client_proc::ClientProc;
use crate::harness::server::EmbeddedServer;

/// The fail-closed identity path, end to end: a certificate whose Common Name is
/// not a valid issued identity must be refused at `accept()`, and the client must
/// recognize the refusal and STOP reconnecting rather than looping against a server
/// whose HTTP endpoint is perfectly healthy.
///
/// An empty gamertag yields the CN `minecraft:`, which `ConnectionClassifier`
/// rejects (a game tag with no player name). That reaches the Rejected branch
/// through the ordinary provisioning path, so no hand-forged certificate is needed.
///
/// Requires both artifacts to be pre-built:
/// * server cdylib: `cargo build -p bedrock-voice-chat-server` in `server/`
/// * e2e harness: `cargo build -p bvc-client-e2e`
#[tokio::test(flavor = "multi_thread")]
async fn refused_identity_stops_the_reconnect_loop() {
    let data_dir = tempfile::tempdir().expect("create temp data dir");

    let rocket_port = EmbeddedServer::free_port_tcp();
    let quic_port = EmbeddedServer::free_port_udp();

    let config_json = EmbeddedServer::config_json(rocket_port, quic_port, data_dir.path());
    let certs_path = data_dir.path().join("certificates");

    let lib = EmbeddedServer::load_library();
    let server =
        EmbeddedServer::start(lib, &config_json, rocket_port, quic_port, &certs_path).await;

    let code = server.login_code("");
    let url = format!("https://127.0.0.1:{}", server.rocket_port());

    let client = ClientProc::spawn("", &code, &url, "test-channel");

    // The terminal state the frontend keys off to show "sign in again" instead of
    // spinning the reconnect loop.
    //
    // Note the client legitimately reports `Connected` first: the certificate is
    // properly CA-signed, so the mTLS handshake and the HTTP channel join both
    // succeed, and the refusal is an application-layer decision taken once the
    // server reads the CN. The contract is that the refusal is TERMINAL, not that
    // the connection never appears to come up.
    client
        .await_ui_event(
            "health",
            |payload| payload.contains("Unauthorized"),
            Duration::from_secs(20),
        )
        .expect("server refusal surfaces as a terminal Unauthorized health event");

    // The whole point of the close code: the client must NOT fall through to the
    // health-check reconnect loop, which would re-dial forever against a server
    // whose HTTP endpoint is perfectly healthy.
    assert!(
        client
            .await_ui_event(
                "health",
                |payload| payload.contains("Reconnecting"),
                Duration::from_secs(3),
            )
            .is_err(),
        "a refused identity must not trigger reconnect attempts"
    );

    client.shutdown();
}
