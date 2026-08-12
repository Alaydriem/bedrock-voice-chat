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

    // The target arrives bare from the control plane and must land under the canonical
    // identity — the key the mixer's gain projection resolves against.
    server.post_control_setvolume("Alice", "Bob", 0.9).await;
    alice
        .await_gain_store(
            |s| s["minecraft:Bob"]["gain"].as_f64().is_some_and(|g| (g - 0.9).abs() < 1e-6),
            Duration::from_secs(10),
        )
        .expect("SetVolume must fire the card-render event with Bob's gain persisted");

    // A case-variant target must resolve onto the existing canonical key — the card keyed
    // "minecraft:Bob" updates; no ghost entry forks under either name form.
    server.post_control_setvolume("Alice", "bob", 0.5).await;
    let store = alice
        .await_gain_store(
            |s| s["minecraft:Bob"]["gain"].as_f64().is_some_and(|g| (g - 0.5).abs() < 1e-6),
            Duration::from_secs(10),
        )
        .expect("a case-variant SetVolume must update the canonical entry");
    assert!(
        store.get("minecraft:bob").is_none() && store.get("bob").is_none(),
        "a case-variant target must not fork a ghost store key: {store}"
    );
}

/// The jukebox rides the per-player preference plane under a reserved target, and two things about
/// that have to hold at once.
///
/// Client side: the delivered action must NOT reach the per-player gain store. That store is what
/// the dashboard builds player cards from, so an entry there renders the jukebox as a person, and
/// nothing at runtime reports the mistake. The assertion is made on the snapshot carried by a LATER
/// player-volume event, which acts as a barrier: control actions arrive over one FIFO channel, so
/// once the second player change is visible the jukebox change has been processed. A first player
/// change goes in ahead of it as a positive control — without one, an absence assertion would pass
/// just as well against dead plumbing.
///
/// Server side: the reserved target must survive identity resolution and the echo clamp unchanged.
/// 1.5 rather than a round number because that is the ceiling — the old 1.0 clamp would silently
/// serve this back as 100%, and `minecraft:#jukebox` would mean composition leaked in.
///
/// Honest limitation: ClientAction delivery rides unreliable QUIC datagrams, so a dropped jukebox
/// datagram makes the absence assertion pass vacuously. It fails safe, never falsely red.
#[tokio::test(flavor = "multi_thread")]
async fn a_jukebox_volume_reaches_the_reserved_target_and_never_the_gain_store() {
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

    let alice = ClientProc::spawn("Alice", &alice_code, &url, "ctljuke");
    alice
        .await_connected(Duration::from_secs(30))
        .expect("Alice connects");

    let jukebox = common::consts::audio::JUKEBOX_CONTROL_TARGET;

    server.post_control_setvolume("Alice", "Bob", 0.9).await;
    alice
        .await_gain_store(
            |s| s["minecraft:Bob"]["gain"].as_f64().is_some_and(|g| (g - 0.9).abs() < 1e-6),
            Duration::from_secs(10),
        )
        .expect("the gain-store observable must be live for the absence check below to mean anything");

    server.post_control_setvolume("Alice", jukebox, 1.5).await;

    server.post_control_setvolume("Alice", "Bob", 0.4).await;
    let store = alice
        .await_gain_store(
            |s| s["minecraft:Bob"]["gain"].as_f64().is_some_and(|g| (g - 0.4).abs() < 1e-6),
            Duration::from_secs(10),
        )
        .expect("the barrier change must land");

    assert!(
        store.get(jukebox).is_none() && store.get("minecraft:#jukebox").is_none(),
        "the jukebox must never enter the per-player gain store: {store}"
    );

    let pref = server
        .await_preference(
            "Alice",
            jukebox,
            |p| p["volume"].as_f64().is_some_and(|v| (v - 1.5).abs() < 1e-6),
            Duration::from_secs(10),
        )
        .await;
    assert!(
        pref.is_some(),
        "the jukebox level must reach the preference cache under the reserved target, at 1.5"
    );

    let composed = server
        .await_preference(
            "Alice",
            "minecraft:#jukebox",
            |_| true,
            Duration::from_secs(2),
        )
        .await;
    assert!(
        composed.is_none(),
        "the reserved target must never be composed against a game"
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

    // `await_preference` queries `/api/preferences?owner=Alice`, which the route composes into
    // `minecraft:Alice`. The TARGET stays as the client wrote it, which is now canonical.
    let pref = server
        .await_preference(
            "Alice",
            "minecraft:Bob",
            |p| p["volume"].as_f64().is_some_and(|v| (v - 0.5).abs() < 1e-6),
            Duration::from_secs(10),
        )
        .await;
    assert!(
        pref.is_some(),
        "reporter must push Alice's volume preference for Bob to the server cache"
    );
}
