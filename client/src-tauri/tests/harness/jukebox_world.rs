use std::time::Duration;

use tempfile::TempDir;

use crate::harness::client_proc::ClientProc;
use crate::harness::server::EmbeddedServer;

/// Boots an embedded server and two connected, channel-less clients (Alice, Bob)
/// for jukebox scenarios. Holds the TempDir + server so they outlive the clients.
pub struct JukeboxWorld {
    pub server: EmbeddedServer,
    pub alice: ClientProc,
    pub bob: ClientProc,
    _data_dir: TempDir,
}

impl JukeboxWorld {
    pub async fn boot() -> Self {
        let data_dir = tempfile::tempdir().expect("create temp data dir");
        let rocket_port = EmbeddedServer::free_port_tcp();
        let quic_port = EmbeddedServer::free_port_udp();
        let config_json = EmbeddedServer::config_json(rocket_port, quic_port, data_dir.path());
        let certs_path = data_dir.path().join("certificates");
        let lib = EmbeddedServer::load_library();
        let server =
            EmbeddedServer::start(lib, &config_json, rocket_port, quic_port, &certs_path).await;

        let url = format!("https://127.0.0.1:{}", server.rocket_port());
        let alice_code = server.login_code("Alice");
        let bob_code = server.login_code("Bob");

        // Empty channel name → Connector skips channel join (jukebox is positional).
        let alice = ClientProc::spawn("Alice", &alice_code, &url, "");
        alice
            .await_connected(Duration::from_secs(30))
            .expect("Alice connects");
        let bob = ClientProc::spawn("Bob", &bob_code, &url, "");
        bob.await_connected(Duration::from_secs(30))
            .expect("Bob connects");

        Self {
            server,
            alice,
            bob,
            _data_dir: data_dir,
        }
    }
}
