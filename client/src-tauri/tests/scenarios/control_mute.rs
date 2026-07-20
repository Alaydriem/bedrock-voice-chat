use std::time::Duration;

use crate::harness::client_proc::ClientProc;
use crate::harness::server::EmbeddedServer;

/// A ClientBound `ClientAction` (SetMuted) delivered over the control plane must mute
/// the actor's own input device, end to end: the server's `route_self` delivers it to
/// Alice's QUIC connection, the output router's `PacketType::ClientAction` arm hands it
/// to the `ControlActionsManager`, which drives the `AudioActionsManager`.
///
/// Alice connects, then a `POST /api/control` (exactly as a mod does) drives
/// `SetMuted(true)` attributed to her; the harness asserts her client reports muted.
///
/// Requires both artifacts to be pre-built:
/// * server cdylib: `cargo build -p bedrock-voice-chat-server` in `server/`
/// * e2e harness: `cargo build -p bvc-client-e2e`
#[tokio::test(flavor = "multi_thread")]
async fn client_action_setmuted_mutes_the_actor() {
    let data_dir = tempfile::tempdir().expect("create temp data dir");

    let rocket_port = EmbeddedServer::free_port_tcp();
    let quic_port = EmbeddedServer::free_port_udp();

    let config_json = EmbeddedServer::config_json(rocket_port, quic_port, data_dir.path());
    let certs_path = data_dir.path().join("certificates");

    let lib = EmbeddedServer::load_library();
    let server =
        EmbeddedServer::start(lib, &config_json, rocket_port, quic_port, &certs_path).await;

    let alice_code = server.login_code("Alice");
    let url = format!("https://127.0.0.1:{}", server.rocket_port());

    let alice = ClientProc::spawn("Alice", &alice_code, &url, "ctl");
    alice
        .await_connected(Duration::from_secs(30))
        .expect("Alice connects");

    // The reporter emits an initial snapshot; she starts unmuted.
    alice
        .await_muted(false, Duration::from_secs(5))
        .expect("Alice starts unmuted");

    // Drive a self-action into the plane over HTTP, actor = Alice.
    server.post_control_setmuted("Alice", true).await;

    alice
        .await_muted(true, Duration::from_secs(5))
        .expect("Alice's input device must be muted after the ClientBound ClientAction");
}
