use std::time::Duration;

use crate::harness::client_proc::ClientProc;
use crate::harness::server::EmbeddedServer;

/// A local audio-state change must reach the server's control cache without any
/// polling by the mod: the client's `QueryStateReporter` pushes a ServerBound
/// `QueryState` whenever the self-state changes, and a snapshot on connect so a
/// fresh player's `/api/state` is never `None`.
///
/// Alice connects (the connect snapshot must seed the cache with her unmuted
/// state), then a `POST /api/control` drives `SetMuted(true)` through the
/// ClientBound plane; once her client applies it, the reporter must push the
/// changed state back so `GET /api/state?id=Alice` reflects `muted == true`.
///
/// Requires both artifacts to be pre-built:
/// * server cdylib: `cargo build -p bedrock-voice-chat-server` in `server/`
/// * e2e harness: `cargo build -p bvc-client-e2e`
#[tokio::test(flavor = "multi_thread")]
async fn local_state_change_reaches_server_cache() {
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

    let alice = ClientProc::spawn("Alice", &alice_code, &url, "ctlrep");
    alice
        .await_connected(Duration::from_secs(30))
        .expect("Alice connects");

    // The connect snapshot must seed the cache before any action fires.
    let initial = server
        .await_state(
            "Alice",
            |s| s["muted"] == serde_json::Value::Bool(false),
            Duration::from_secs(10),
        )
        .await;
    assert!(
        initial.is_some(),
        "connect snapshot must seed /api/state with Alice's unmuted self-state"
    );

    // Mute Alice over the control plane; her client applies it locally and the
    // reporter must push the new state back to the server cache.
    server.post_control_setmuted("Alice", true).await;

    let after = server
        .await_state(
            "Alice",
            |s| s["muted"] == serde_json::Value::Bool(true),
            Duration::from_secs(10),
        )
        .await;
    assert!(
        after.is_some(),
        "reporter must push muted=true to the server cache after the ClientAction lands"
    );
}

/// A delivered volume action must fire the `player_gain_store_updated` event —
/// the exact trigger the dashboard's player cards re-render on — carrying a
/// persisted store a card could render, and a case-variant follow-up must land
/// on the SAME canonical entry instead of forking a ghost key (which would play
/// no audio and render on no card).
#[tokio::test(flavor = "multi_thread")]
async fn volume_action_fires_card_render_event_with_canonical_entry() {
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

    let alice = ClientProc::spawn("Alice", &alice_code, &url, "ctlcard");
    alice
        .await_connected(Duration::from_secs(30))
        .expect("Alice connects");

    // First action establishes the entry under its exact key and must fire the
    // card-render event with the persisted value.
    server.post_control_setvolume("Alice", "Bob", 0.9).await;
    alice
        .await_gain_store(
            |s| s["Bob"]["gain"].as_f64().is_some_and(|g| (g - 0.9).abs() < 1e-6),
            Duration::from_secs(10),
        )
        .expect("SetVolume must fire the card-render event with Bob's gain persisted");

    // A case-variant target must resolve onto the existing canonical key — the
    // card keyed "Bob" updates; no ghost "bob" entry forks.
    server.post_control_setvolume("Alice", "bob", 0.5).await;
    let store = alice
        .await_gain_store(
            |s| s["Bob"]["gain"].as_f64().is_some_and(|g| (g - 0.5).abs() < 1e-6),
            Duration::from_secs(10),
        )
        .expect("a case-variant SetVolume must update the canonical entry");
    assert!(
        store.get("bob").is_none(),
        "a case-variant target must not fork a ghost store key: {store}"
    );
}

/// A per-player preference change must reach the server's preference cache: a
/// `SetVolume` ClientAction lands on Alice's client, the facilitator writes the
/// gain through the persisted `player_gain_store` path, and the reporter pushes
/// a ServerBound `PlayerPreference` so `GET /api/preferences?owner=Alice&targets=Bob`
/// reflects the new volume.
#[tokio::test(flavor = "multi_thread")]
async fn preference_change_reaches_server_cache() {
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

    let alice = ClientProc::spawn("Alice", &alice_code, &url, "ctlrep");
    alice
        .await_connected(Duration::from_secs(30))
        .expect("Alice connects");

    server.post_control_setvolume("Alice", "Bob", 0.5).await;

    let pref = server
        .await_preference(
            "Alice",
            "Bob",
            |p| p["volume"].as_f64().is_some_and(|v| (v - 0.5).abs() < 1e-6),
            Duration::from_secs(10),
        )
        .await;
    assert!(
        pref.is_some(),
        "reporter must push Alice's volume preference for Bob to the server cache"
    );
}
