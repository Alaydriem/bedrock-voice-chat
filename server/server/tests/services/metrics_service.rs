use bvc_server_lib::services::MetricsService;
use bvc_server_lib::services::metrics_service::event::TelemetryEvent;
use bvc_server_lib::services::metrics_service::posthog::client::PosthogClient;
use chrono::TimeZone;

fn ca_dir(name: &str) -> String {
    let dir = std::env::temp_dir().join(name);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("ca.crt"), b"stable-ca-bytes").unwrap();
    dir.to_string_lossy().to_string()
}

#[test]
fn server_id_is_deterministic_for_same_ca_bytes() {
    let path = ca_dir("bvc-metrics-serverid-ca");
    let a = MetricsService::derive_server_id(&path);
    let b = MetricsService::derive_server_id(&path);
    assert_eq!(a, b);
    assert!(!a.is_empty());
}

#[test]
fn build_batch_maps_events_without_player_identity() {
    let client = PosthogClient::new(
        "https://us.i.posthog.com".to_string(),
        "phc_x".to_string(),
        "server-abc".to_string(),
        "1.2.3".to_string(),
    );
    let t1 = chrono::Utc.with_ymd_and_hms(2026, 7, 6, 10, 0, 0).unwrap();
    let t2 = chrono::Utc.with_ymd_and_hms(2026, 7, 6, 10, 5, 0).unwrap();

    let batch = client.build_batch(&[
        TelemetryEvent::ServerStarted {
            at: t1,
            hostname_sha: "abc123".to_string(),
        },
        TelemetryEvent::Connected { at: t1 },
        TelemetryEvent::Disconnected {
            at: t2,
            duration_secs: 42,
        },
        TelemetryEvent::ChannelJoined { at: t1 },
        TelemetryEvent::ChannelLeft { at: t2 },
    ]);

    let json = serde_json::to_string(&batch).unwrap();
    assert!(json.contains("server_started"));
    assert!(json.contains("player_connected"));
    assert!(json.contains("player_disconnected"));
    assert!(json.contains("channel_joined"));
    assert!(json.contains("channel_left"));
    assert!(json.contains("\"distinct_id\":\"server-abc\""));
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
    let (svc, _posthog) = MetricsService::new_shared(false, &path, "/nonexistent-cert.pem");

    svc.record_connect();
    svc.record_disconnect(std::time::Duration::from_secs(5));
    svc.record_channel_join();
    svc.record_channel_leave();
    svc.set_active_players(7);
    svc.set_active_channels(2);
    svc.set_players_in_channels(3);

    let body = svc.render();
    for family in [
        "bvc_build_info",
        "bvc_player_connections_total",
        "bvc_player_disconnections_total",
        "bvc_session_duration_seconds",
        "bvc_channel_joins_total",
        "bvc_channel_leaves_total",
        "bvc_active_players",
        "bvc_active_channels",
        "bvc_players_in_channels",
    ] {
        assert!(body.contains(family), "missing {family} in exposition:\n{body}");
    }
    assert!(!body.to_lowercase().contains("gamertag"));
}

// G7: telemetry disabled ⇒ no PostHog task is spawned (the opt-out control).
#[tokio::test]
async fn disabled_telemetry_spawns_no_posthog_task() {
    let path = ca_dir("bvc-metrics-disabled-ca");
    let (_svc, handle) = MetricsService::new_shared(false, &path, "/nonexistent-cert.pem");
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

    let client = PosthogClient::new(host, "phc_x".to_string(), "server-abc".to_string(), "1.2.3".to_string());
    let (tx, rx) = tokio::sync::mpsc::channel(8);
    let run = tokio::spawn(async move {
        client
            .run(rx, tokio_util::sync::CancellationToken::new())
            .await
    });

    tx.send(TelemetryEvent::Connected { at: chrono::Utc::now() })
        .await
        .unwrap();
    tx.send(TelemetryEvent::ChannelJoined { at: chrono::Utc::now() })
        .await
        .unwrap();
    drop(tx); // closing the channel triggers the final flush, then run exits

    let _ = tokio::time::timeout(std::time::Duration::from_secs(3), run).await;
    let req = tokio::time::timeout(std::time::Duration::from_secs(3), server)
        .await
        .unwrap()
        .unwrap();

    assert!(req.contains("POST /batch/"), "request:\n{req}");
    assert!(req.contains("player_connected"), "request:\n{req}");
    assert!(req.contains("phc_x"));
    assert!(!req.to_lowercase().contains("gamertag"));
}
