use crate::harness::server::EmbeddedServer;

/// Boot the real BVC server in-process and provision a single-use login code
/// for a player via the `bvc_provision_login_code` FFI. Proves the orchestrator
/// can mint a code that a client could later redeem through `code_login`.
///
/// Requires the server cdylib to be built first:
/// `cargo build -p bedrock-voice-chat-server` in the `server/` workspace.
#[tokio::test(flavor = "multi_thread")]
async fn provisions_single_use_login_code() {
    let data_dir = tempfile::tempdir().expect("create temp data dir");

    let rocket_port = EmbeddedServer::free_port_tcp();
    let quic_port = EmbeddedServer::free_port_udp();

    let config_json = EmbeddedServer::config_json(rocket_port, quic_port, data_dir.path());
    let certs_path = data_dir.path().join("certificates");

    let lib = EmbeddedServer::load_library();
    let server =
        EmbeddedServer::start(lib, &config_json, rocket_port, quic_port, &certs_path).await;

    let code = server.login_code("Alice");

    // The exact length is a server-side detail; assert a sane, non-trivial range
    // rather than coupling the test to one specific code length.
    assert!(
        (6..=64).contains(&code.len()),
        "expected a login code of 6-64 chars, got {} chars: {code}",
        code.len()
    );
    assert!(
        code.chars().all(|c| c.is_ascii_alphanumeric()),
        "login code should be alphanumeric, got: {code}"
    );
}
