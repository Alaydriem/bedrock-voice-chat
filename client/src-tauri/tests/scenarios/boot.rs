use crate::harness::server::EmbeddedServer;

/// Boot the real BVC server in-process via the FFI cdylib on free ports and
/// assert it serves `/api/config`. Proves the harness can stand up the full
/// server stack (Rocket + QUIC + SQLite migrations) for later scenarios.
///
/// Requires the server cdylib to be built first:
/// `cargo build -p bedrock-voice-chat-server` in the `server/` workspace.
#[tokio::test(flavor = "multi_thread")]
async fn server_boots_and_serves_config() {
    let data_dir = tempfile::tempdir().expect("create temp data dir");

    let rocket_port = EmbeddedServer::free_port_tcp();
    let quic_port = EmbeddedServer::free_port_udp();

    let config_json = EmbeddedServer::config_json(rocket_port, quic_port, data_dir.path());
    let certs_path = data_dir.path().join("certificates");

    let lib = EmbeddedServer::load_library();
    let server =
        EmbeddedServer::start(lib, &config_json, rocket_port, quic_port, &certs_path).await;

    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .expect("build reqwest client");

    let resp = client
        .get(format!(
            "https://127.0.0.1:{}/api/config",
            server.rocket_port()
        ))
        .send()
        .await
        .expect("GET /api/config");

    assert!(
        resp.status().is_success(),
        "expected /api/config to succeed, got {}",
        resp.status()
    );

    let body: serde_json::Value = resp.json().await.expect("parse /api/config json");
    assert_eq!(body["status"], "Ok");
    assert_eq!(body["quic_port"], quic_port as u64);
}
