use std::sync::Arc;
use std::time::Duration;

use tokio_util::sync::CancellationToken;

use bvc_server_lib::config::Meridian;
use bvc_server_lib::services::MeridianService;

fn config(name: &str) -> Meridian {
    Meridian {
        // Closed port: every registration attempt will fail.
        url: "https://127.0.0.1:1".to_string(),
        api_key: "k".to_string(),
        instance_id: 42,
        name: name.to_string(),
        host: None,
        backend: "127.0.0.1".to_string(),
    }
}

fn service(name: &str) -> MeridianService {
    MeridianService::new(
        config(name),
        "127.0.0.1".to_string(),
        443,
        4433,
        "x.example.com".to_string(),
    )
}

#[test]
fn record_name_comes_from_config_and_is_stable() {
    let a = service("customer-x");
    let b = service("customer-x");
    assert_eq!(a.record_name(), "customer-x");
    assert_eq!(
        a.record_name(),
        b.record_name(),
        "two services with the same config must use the same record name, \
         otherwise re-registration leaks a registry entry per restart"
    );
}

#[tokio::test]
async fn heartbeat_survives_registration_failures() {
    let svc = Arc::new(service("customer-x"));
    let shutdown = CancellationToken::new();
    let handle = svc.spawn_heartbeat(shutdown.clone());

    // Long enough for at least one tick against the closed port.
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert!(
        !handle.is_finished(),
        "the heartbeat must survive failures — a Meridian restart is exactly \
         the case it exists for"
    );

    shutdown.cancel();
    tokio::time::timeout(Duration::from_secs(2), handle)
        .await
        .expect("heartbeat must stop on shutdown")
        .unwrap();
}
