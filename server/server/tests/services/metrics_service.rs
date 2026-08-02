use bvc_server_lib::runtime::ca_cert::CaCertManager;
use bvc_server_lib::services::MetricsService;
use bvc_server_lib::services::metrics_service::event::TelemetryEvent;
use bvc_server_lib::services::metrics_service::heartbeat_snapshot::HeartbeatSnapshot;
use bvc_server_lib::services::metrics_service::posthog::client::PosthogClient;
use chrono::TimeZone;

fn ca_dir(name: &str) -> String {
    ca_dir_with_sans(name, &[String::from("localhost")])
}

fn ca_dir_with_sans(name: &str, sans: &[String]) -> String {
    let dir = std::env::temp_dir().join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.to_string_lossy().to_string();
    CaCertManager::new(&path).ensure(sans).unwrap();
    path
}

#[test]
fn server_id_is_deterministic_for_same_ca() {
    let path = ca_dir("bvc-metrics-serverid-ca");
    let a = MetricsService::derive_server_id(&path);
    let b = MetricsService::derive_server_id(&path);
    assert_eq!(a, b);
    assert!(!a.is_empty());
}

// The identity must survive a SAN change. `ensure` re-signs ca.crt when the
// configured SAN set drifts, which previously produced a brand-new server_id and
// silently forked one deployment into two.
#[test]
fn server_id_survives_a_san_re_sign() {
    let path = ca_dir_with_sans("bvc-metrics-serverid-drift", &[String::from("a.example")]);
    let before = MetricsService::derive_server_id(&path);

    CaCertManager::new(&path)
        .ensure(&[String::from("a.example"), String::from("b.example")])
        .unwrap();
    let after = MetricsService::derive_server_id(&path);

    assert_eq!(
        before, after,
        "adding a SAN re-signs ca.crt but must not change the server identity"
    );
}

#[test]
fn build_batch_maps_events_without_player_identity() {
    let client = PosthogClient::new(
        "https://us.i.posthog.com".to_string(),
        "phc_x".to_string(),
        "server-abc".to_string(),
        "1.2.3".to_string(),
        "abc123".to_string(),
    );
    let t1 = chrono::Utc.with_ymd_and_hms(2026, 7, 6, 10, 0, 0).unwrap();
    let t2 = chrono::Utc.with_ymd_and_hms(2026, 7, 6, 10, 5, 0).unwrap();

    let batch = client.build_batch(&[
        TelemetryEvent::ServerStarted { at: t1 },
        TelemetryEvent::PlayerConnected { at: t1 },
        TelemetryEvent::PlayerDisconnected {
            at: t2,
            duration_secs: 42,
        },
        TelemetryEvent::ChannelJoined { at: t1 },
        TelemetryEvent::ChannelLeft { at: t2 },
    ]);

    let json = serde_json::to_string(&batch).unwrap();
    assert!(json.contains("Server::Started"));
    assert!(json.contains("Server::PlayerConnected"));
    assert!(json.contains("Server::PlayerDisconnected"));
    assert!(json.contains("Server::ChannelJoined"));
    assert!(json.contains("Server::ChannelLeft"));
    assert!(json.contains("\"distinct_id\":\"server-abc\""));
    assert!(json.contains("\"server_id\":\"server-abc\""));
    assert!(json.contains("\"hostname_sha\":\"abc123\""));
    assert!(json.contains("\"session_duration_secs\":42"));
    // per-event occurrence timestamps, not a shared flush time
    assert!(json.contains("2026-07-06T10:00:00"));
    assert!(json.contains("2026-07-06T10:05:00"));
    // privacy invariant: no player identity ever leaves
    assert!(!json.to_lowercase().contains("gamertag"));
}

// End-to-end render: emit through the metrics-rs fanout and assert the real
// PrometheusHandle exposition contains every family, with no player identity.
#[tokio::test]
async fn render_exposes_all_metric_families_without_identity() {
    let path = ca_dir("bvc-metrics-render-ca");
    let (svc, _posthog) = MetricsService::new_shared(false, &path, "/nonexistent-cert.pem", Vec::new(), false);

    svc.record_connect("alice");
    svc.record_disconnect("alice", std::time::Duration::from_secs(5));
    svc.record_channel_join();
    svc.record_channel_leave();
    svc.set_active_players(7);
    svc.set_active_channels(2);
    svc.set_players_in_channels(3);
    svc.record_position_datagram(84, 1);
    svc.record_position_oversize_drop();

    let body = svc.render();
    for family in [
        "bvc_build_info",
        "bvc_player_connections_total",
        "bvc_player_disconnections_total",
        "bvc_session_duration_seconds",
        "bvc_channel_joins_total",
        "bvc_channel_leaves_total",
        "bvc_active_players",
        "bvc_peak_players",
        "bvc_active_channels",
        "bvc_players_in_channels",
        // The position feed is observable independently of audio: proximity
        // gating reads the cache /api/position fills over HTTP, so frames keep
        // routing even when position delivery to clients has failed entirely.
        "bvc_position_datagrams_total",
        "bvc_position_datagram_bytes",
        "bvc_position_players_advertised_total",
        "bvc_position_oversize_drops_total",
    ] {
        assert!(body.contains(family), "missing {family} in exposition:\n{body}");
    }
    assert!(!body.to_lowercase().contains("gamertag"));
}

// G7: telemetry disabled ⇒ no PostHog task is spawned (the opt-out control).
#[tokio::test]
async fn disabled_telemetry_spawns_no_posthog_task() {
    let path = ca_dir("bvc-metrics-disabled-ca");
    let (_svc, handle) = MetricsService::new_shared(false, &path, "/nonexistent-cert.pem", Vec::new(), false);
    assert!(handle.is_none());
}

// G2: exercise the real PostHog run/flush/HTTP path against a local listener.
#[tokio::test]
async fn posthog_client_flushes_events_over_http() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let host = format!("http://{}", addr);

    let server = tokio::spawn(async move {
        let (mut sock, _) = listener.accept().await.unwrap();
        let mut buf = vec![0u8; 8192];
        let n = sock.read(&mut buf).await.unwrap();
        sock.write_all(b"HTTP/1.1 200 OK\r\ncontent-length: 0\r\n\r\n")
            .await
            .unwrap();
        String::from_utf8_lossy(&buf[..n]).to_string()
    });

    let client = PosthogClient::new(
        host,
        "phc_x".to_string(),
        "server-abc".to_string(),
        "1.2.3".to_string(),
        "abc123".to_string(),
    );
    let (tx, rx) = tokio::sync::mpsc::channel(8);
    let run = tokio::spawn(async move {
        client
            .run(rx, tokio_util::sync::CancellationToken::new())
            .await
    });

    tx.send(TelemetryEvent::PlayerConnected {
        at: chrono::Utc::now(),
    })
    .await
    .unwrap();
    tx.send(TelemetryEvent::ChannelJoined { at: chrono::Utc::now() })
        .await
        .unwrap();
    tx.send(TelemetryEvent::Heartbeat {
        at: chrono::Utc::now(),
        snapshot: HeartbeatSnapshot {
            uptime_secs: 1234,
            window_secs: 900,
            player_count: 5,
            peak_player_count: 9,
            players_reached: 3,
            players_reached_proximity: 2,
            players_reached_channel: 2,
            players_reached_mutual: 2,
            players_reached_mutual_proximity: 2,
            players_reached_mutual_channel: 0,
            features_enabled: vec!["telemetry".to_string()],
        },
    })
    .await
    .unwrap();
    drop(tx); // closing the channel triggers the final flush, then run exits

    let _ = tokio::time::timeout(std::time::Duration::from_secs(3), run).await;
    let req = tokio::time::timeout(std::time::Duration::from_secs(3), server)
        .await
        .unwrap()
        .unwrap();

    assert!(req.contains("POST /batch/"), "request:\n{req}");
    assert!(req.contains("Server::PlayerConnected"), "request:\n{req}");
    assert!(req.contains("phc_x"));
    assert!(!req.to_lowercase().contains("gamertag"));
    // The snapshot is flattened onto the event, not nested under a "heartbeat" key —
    // a nested payload would leave every heartbeat property unqueryable in PostHog.
    assert!(req.contains("\"uptime_secs\":1234"), "request:\n{req}");
    assert!(req.contains("\"peak_player_count\":9"), "request:\n{req}");
    assert!(!req.contains("\"heartbeat\":"), "request:\n{req}");
}

#[tokio::test]
async fn peak_players_holds_the_high_water_mark() {
    let path = ca_dir("bvc-metrics-peak-ca");
    let (svc, _posthog) = MetricsService::new_shared(false, &path, "/nonexistent-cert.pem", Vec::new(), false);

    svc.set_active_players(3);
    svc.set_active_players(9);
    svc.set_active_players(2);

    assert_eq!(svc.active_players(), 2);
    assert_eq!(svc.peak_players(), 9);
}

// The daily reset drops the high-water mark to whoever is online at the moment of
// the reset, not to zero — a server busy across midnight has not gone empty.
#[tokio::test]
async fn resetting_peak_players_drops_to_the_current_count() {
    let path = ca_dir("bvc-metrics-peak-reset-ca");
    let (svc, _posthog) = MetricsService::new_shared(false, &path, "/nonexistent-cert.pem", Vec::new(), false);

    svc.set_active_players(9);
    svc.set_active_players(4);
    svc.reset_peak_players();

    assert_eq!(svc.peak_players(), 4);
}

// The heartbeat's player_count and the bvc_active_players gauge must be the same
// number; they are written by one method so they cannot drift.
#[tokio::test]
async fn heartbeat_closes_the_window_and_publishes_interaction_gauges() {
    use bvc_server_lib::services::metrics_service::interaction::InteractionRoute;
    use bvc_server_lib::services::metrics_service::interaction::InteractionTracker;

    let path = ca_dir("bvc-metrics-heartbeat-ca");
    let (svc, _posthog) = MetricsService::new_shared(
        false,
        &path,
        "/nonexistent-cert.pem",
        vec!["telemetry".to_string()],
        false,
    );

    svc.set_active_players(5);
    svc.record_interaction(
        InteractionRoute::Proximity,
        InteractionTracker::hash_name("alice"),
        InteractionTracker::hash_name("bob"),
    );

    let mut last = chrono::Utc::now().date_naive();
    svc.emit_heartbeat(&mut last);

    let body = svc.render();
    assert!(body.contains("bvc_players_reached"), "exposition:\n{body}");
    assert!(body.contains("route=\"proximity\""), "exposition:\n{body}");

    // the window closed, so the next one starts empty
    assert_eq!(
        svc.interactions()
            .counts(InteractionRoute::Proximity)
            .reached,
        0
    );
    assert!(!body.to_lowercase().contains("gamertag"));
}

// A day boundary resets the high-water mark to whoever is online, and does not
// disturb the live count.
#[tokio::test]
async fn heartbeat_resets_peak_on_a_new_utc_day() {
    let path = ca_dir("bvc-metrics-heartbeat-day-ca");
    let (svc, _posthog) =
        MetricsService::new_shared(false, &path, "/nonexistent-cert.pem", Vec::new(), false);

    svc.set_active_players(9);
    svc.set_active_players(2);

    let mut last = chrono::Utc::now().date_naive() - chrono::Duration::days(1);
    svc.emit_heartbeat(&mut last);

    assert_eq!(svc.peak_players(), 2);
    assert_eq!(svc.active_players(), 2);
    assert_eq!(last, chrono::Utc::now().date_naive());
}

// A reconnect inside the window must be distinguishable from a first connect.
// Only the elapsed delta is emitted; the player name is a local cache key.
#[tokio::test]
async fn a_reconnect_within_the_window_is_reported_as_a_reconnect() {
    let path = ca_dir("bvc-metrics-reconnect-ca");
    let (svc, _posthog) =
        MetricsService::new_shared(false, &path, "/nonexistent-cert.pem", Vec::new(), false);

    assert!(!svc.saw_recent_disconnect("alice"));

    svc.record_disconnect("alice", std::time::Duration::from_secs(30));
    assert!(svc.saw_recent_disconnect("alice"));

    svc.record_connect("alice");
    // consumed by the reconnect, so a second connect is a fresh session
    assert!(!svc.saw_recent_disconnect("alice"));
}

#[tokio::test]
async fn an_unrelated_player_connecting_is_not_a_reconnect() {
    let path = ca_dir("bvc-metrics-reconnect-other-ca");
    let (svc, _posthog) =
        MetricsService::new_shared(false, &path, "/nonexistent-cert.pem", Vec::new(), false);

    svc.record_disconnect("alice", std::time::Duration::from_secs(30));
    svc.record_connect("bob");

    assert!(svc.saw_recent_disconnect("alice"));
}

// Shutdown emits Server::Stopped and cancels the drain token with no await in
// between, so the event is always still in the channel when cancellation lands.
// `select!` chooses among ready branches at random, so a drain that stops at the
// cancel would discard it — and a graceful shutdown reporting no Server::Stopped
// reads downstream as a crash. Twenty events make the race deterministic: losing
// none by chance is a one-in-a-million event.
#[tokio::test]
async fn cancelling_the_drain_does_not_discard_queued_events() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let host = format!("http://{}", addr);

    let server = tokio::spawn(async move {
        let (mut sock, _) = listener.accept().await.unwrap();
        let mut buf = vec![0u8; 65536];
        let n = sock.read(&mut buf).await.unwrap();
        sock.write_all(b"HTTP/1.1 200 OK\r\ncontent-length: 0\r\n\r\n")
            .await
            .unwrap();
        String::from_utf8_lossy(&buf[..n]).to_string()
    });

    let client = PosthogClient::new(
        host,
        "phc_x".to_string(),
        "server-abc".to_string(),
        "1.2.3".to_string(),
        "abc123".to_string(),
    );
    let (tx, rx) = tokio::sync::mpsc::channel(64);
    let shutdown = tokio_util::sync::CancellationToken::new();
    let drain = shutdown.clone();
    let run = tokio::spawn(async move { client.run(rx, drain).await });

    for _ in 0..19 {
        tx.try_send(TelemetryEvent::ChannelJoined {
            at: chrono::Utc::now(),
        })
        .unwrap();
    }
    tx.try_send(TelemetryEvent::Stopped {
        at: chrono::Utc::now(),
        uptime_secs: 99,
        stop_reason: "graceful",
    })
    .unwrap();
    shutdown.cancel();

    let _ = tokio::time::timeout(std::time::Duration::from_secs(3), run).await;
    let req = tokio::time::timeout(std::time::Duration::from_secs(3), server)
        .await
        .unwrap()
        .unwrap();

    assert!(req.contains("Server::Stopped"), "request:\n{req}");
    assert!(req.contains("\"stop_reason\":\"graceful\""), "request:\n{req}");
}
