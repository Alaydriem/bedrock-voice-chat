use std::time::Duration;

use futures_util::{SinkExt, StreamExt};

use crate::harness::client_proc::ClientProc;
use crate::harness::server::EmbeddedServer;

const KEY: &str = "e2e-command-socket-key";

/// A deafen sent over the command WebSocket must mute the microphone too, end to end.
///
/// The output device is deafen, and deafen drives both devices — hearing nobody while they
/// can still hear you is a state a user cannot detect from their own screen. The app's own
/// button and the in-game control action both routed through `set_deafened` and got the
/// pair; this surface toggled the output device directly and produced the half-deafen
/// neither of the others can.
///
/// Driven through the real listener a Stream Deck plugin dials, so the command dispatch is
/// under test rather than the action layer beneath it.
///
/// Requires both artifacts to be pre-built:
/// * server cdylib: `cargo build -p bedrock-voice-chat-server` in `server/`
/// * e2e harness: `cargo build -p bvc-client-e2e`
#[tokio::test(flavor = "multi_thread")]
async fn websocket_deafen_also_mutes_the_input() {
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

    let alice = ClientProc::spawn("Alice", &alice_code, &url, "wsdeafen");
    alice
        .await_connected(Duration::from_secs(30))
        .expect("Alice connects");

    // She starts with both flags clear, so the pair the deafen produces is unambiguous.
    alice
        .await_mute_pair((false, false), Duration::from_secs(5))
        .expect("Alice starts neither muted nor deafened");

    let address = alice
        .start_command_websocket(KEY, Duration::from_secs(10))
        .expect("the command WebSocket binds");

    let (mut socket, _) = tokio_tungstenite::connect_async(&address)
        .await
        .expect("a controller connects to the command WebSocket");

    // Exactly the frame a Stream Deck deafen button sends. There is no deafen command: the
    // output device is what the protocol calls it.
    let frame = serde_json::json!({ "action": "mute", "device": "output", "key": KEY });
    socket
        .send(tokio_tungstenite::tungstenite::Message::Text(
            frame.to_string().into(),
        ))
        .await
        .expect("send the deafen command");

    let reply = socket
        .next()
        .await
        .expect("the server answers")
        .expect("the answer is a frame");
    let reply: serde_json::Value =
        serde_json::from_str(reply.to_text().expect("the answer is text"))
            .expect("the answer is JSON");

    assert_eq!(
        reply["success"], true,
        "the deafen command must be accepted: {reply}"
    );
    assert_eq!(
        reply["data"]["device"], "output",
        "the response names the device the caller asked about: {reply}"
    );
    assert_eq!(
        reply["data"]["muted"], true,
        "the response reports the deafen it applied: {reply}"
    );

    // The assertion this test exists for. `deafened` alone passing is the bug.
    alice
        .await_mute_pair((true, true), Duration::from_secs(5))
        .expect("a WebSocket deafen must mute the input as well as the output");

    // The dashboard renders mute state off the mute:input Tauri event, so the input leg has
    // to announce itself rather than only flipping the stream.
    alice
        .await_ui_event("mute:input", |p| p == "true", Duration::from_secs(5))
        .expect("the mute:input render trigger must fire for the paired input mute");

    // Undeafening clears both, because the fix for "I cannot hear anyone" must not leave a
    // muted microphone behind. Alice is in open mic, where an idle microphone belongs open.
    socket
        .send(tokio_tungstenite::tungstenite::Message::Text(
            frame.to_string().into(),
        ))
        .await
        .expect("send the undeafen command");

    alice
        .await_mute_pair((false, false), Duration::from_secs(5))
        .expect("undeafening over WebSocket must clear both flags in open mic");
}
