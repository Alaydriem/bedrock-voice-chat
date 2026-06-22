use std::time::Duration;

use crate::harness::client_proc::ClientProc;
use crate::harness::server::EmbeddedServer;

/// First full end-to-end integration: boot the real BVC server in-process,
/// mint a single-use login code, then spawn the real Wry client (e2e bin) and
/// drive its full connect sequence (code_login -> initialize_api_client ->
/// change_network_stream QUIC mTLS -> channel join) until it emits `Connected`.
///
/// Requires both artifacts to be pre-built:
/// * server cdylib: `cargo build -p bedrock-voice-chat-server` in `server/`
/// * e2e harness: `cargo build -p bvc-client-e2e`
#[tokio::test(flavor = "multi_thread")]
async fn single_client_connects_and_joins() {
    let data_dir = tempfile::tempdir().expect("create temp data dir");

    let rocket_port = EmbeddedServer::free_port_tcp();
    let quic_port = EmbeddedServer::free_port_udp();

    let config_json = EmbeddedServer::config_json(rocket_port, quic_port, data_dir.path());
    let certs_path = data_dir.path().join("certificates");

    let lib = EmbeddedServer::load_library();
    let server =
        EmbeddedServer::start(lib, &config_json, rocket_port, quic_port, &certs_path).await;

    let code = server.login_code("Alice");

    // The server cert SAN carries `localhost` and `127.0.0.1`; the QUIC connect
    // derives `server_fqdn` from this URL's host, so it must match a SAN entry.
    let url = format!("https://127.0.0.1:{}", server.rocket_port());

    let client = ClientProc::spawn("Alice", &code, &url, "test-channel");

    client
        .await_connected(Duration::from_secs(20))
        .expect("client connects + joins");

    client.shutdown();
}
