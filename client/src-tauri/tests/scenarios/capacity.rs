use std::time::Duration;

use crate::harness::client_proc::ClientProc;
use crate::harness::server::EmbeddedServer;

/// The concurrent-session limit, end to end, on a server that admits exactly one player.
///
/// ## What this proves
///
/// Four things, each needing a real client on the far end of a real refusal:
///
/// 1. The limit is enforced. With `voice.limits.connections = 1`, the second player is
///    refused while the first holds the slot.
/// 2. The refusal reaches the client as `AtCapacity` rather than as a bare close. A
///    transport close carries no reason, so the client could not tell a full server from a
///    revoked credential, and would tell the player the wrong thing.
/// 3. The refusal is NOT terminal. This is the exact inverse of
///    `unauthorized::refused_identity_stops_the_reconnect_loop`, where a refused identity
///    must stop re-dialling: refused-because-full must keep going, or a player would sit
///    on a full server forever while slots came free around them.
/// 4. A departed player's slot is released to somebody else. Alice's slot is *reserved* for
///    the 60 s default grace when her client closes cleanly, and Carol is a new identity
///    with nothing free, so Carol is admitted only because a held slot yields to real
///    demand. Without that displacement rule this last step would hang until the grace
///    expired.
///
/// ## Assertion strategy
///
/// Liveness is read from the health event stream, not from the diagnostics snapshot. The
/// snapshot is derived from sending with nothing returning — see `link_stall.rs` — so an
/// idle client that has just been refused goes on reporting `connected` until it next
/// transmits. Bob is silent, so his snapshot says connected long after his session ended,
/// and an assertion on it would pass against the state from before the refusal.
///
/// Step 4 uses a third client rather than reusing Bob for the same reason in reverse: Bob's
/// `connected` latch is already set from the moment before he was refused, so his admission
/// cannot be distinguished from his first connect. Carol is a fresh process, so her latch
/// is unambiguous. Bob is shut down before Alice so the two of them cannot race for the one
/// slot that frees.
///
/// Requires both artifacts to be pre-built:
/// * server cdylib: `cargo build -p bedrock-voice-chat-server` in `server/`
/// * e2e harness: `cargo build -p bvc-client-e2e`
#[tokio::test(flavor = "multi_thread")]
async fn a_one_player_server_refuses_the_second_and_frees_the_slot_when_the_first_leaves() {
    let data_dir = tempfile::tempdir().expect("create temp data dir");

    let rocket_port = EmbeddedServer::free_port_tcp();
    let quic_port = EmbeddedServer::free_port_udp();

    let config_json =
        EmbeddedServer::config_json_with_capacity(rocket_port, quic_port, data_dir.path(), 1);
    let certs_path = data_dir.path().join("certificates");

    let lib = EmbeddedServer::load_library();
    let server =
        EmbeddedServer::start(lib, &config_json, rocket_port, quic_port, &certs_path).await;

    let alice_code = server.login_code("Alice");
    let bob_code = server.login_code("Bob");
    let carol_code = server.login_code("Carol");

    let url = format!("https://127.0.0.1:{}", server.rocket_port());

    let alice = ClientProc::spawn("Alice", &alice_code, &url, "capacity-test");
    alice
        .await_connected(Duration::from_secs(30))
        .expect("Alice takes the only slot");

    let bob = ClientProc::spawn("Bob", &bob_code, &url, "capacity-test");

    // Bob legitimately reports Connected first: his certificate is properly signed, so mTLS
    // and the HTTP channel join both succeed, and the refusal is the admission check taken
    // after that. The contract is that he is refused, not that he never appears to come up.
    bob.await_ui_event(
        "health",
        |payload| payload.contains("AtCapacity"),
        Duration::from_secs(30),
    )
    .expect("the second player is refused, and told that the server is full");

    bob.await_ui_event(
        "health",
        |payload| payload.contains("Reconnecting"),
        Duration::from_secs(60),
    )
    .expect("a full server is not terminal: the refused player keeps trying");

    // Registration stays open, so Bob reached the roster and the HTTP API before being
    // refused the voice session. None of that may cost Alice the slot she holds.
    alice
        .await_diagnostics(|(connected, _, _)| *connected, Duration::from_secs(10))
        .expect("Alice keeps her session while Bob is turned away");

    bob.shutdown();
    alice.shutdown();

    let carol = ClientProc::spawn("Carol", &carol_code, &url, "capacity-test");
    carol
        .await_connected(Duration::from_secs(60))
        .expect("the slot Alice released admits the next player, with no operator action");

    carol.shutdown();
}
