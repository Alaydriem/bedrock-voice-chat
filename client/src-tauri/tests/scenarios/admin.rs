use crate::harness::server::EmbeddedServer;

struct AdminScenario {
    server: EmbeddedServer,
    _data_dir: tempfile::TempDir,
}

impl AdminScenario {
    async fn start() -> Self {
        let data_dir = tempfile::tempdir().expect("create temp data dir");
        let rocket_port = EmbeddedServer::free_port_tcp();
        let quic_port = EmbeddedServer::free_port_udp();
        let config_json = EmbeddedServer::config_json(rocket_port, quic_port, data_dir.path());
        let certs_path = data_dir.path().join("certificates");
        let lib = EmbeddedServer::load_library();
        let server =
            EmbeddedServer::start(lib, &config_json, rocket_port, quic_port, &certs_path).await;
        Self {
            server,
            _data_dir: data_dir,
        }
    }

    fn admin(&self, gamertag: &str, game: &str, action: &str) -> (i32, Option<String>) {
        let request = serde_json::json!({ "gamertag": gamertag, "game": game, "action": action });
        self.server.admin(&request.to_string())
    }
}

/// The request shape and return codes are the contract the Java mod's `AdminRequest` and
/// `AdminResult` decode.
#[tokio::test(flavor = "multi_thread")]
async fn admin_requests_return_the_documented_codes() {
    let scenario = AdminScenario::start().await;

    assert_eq!(scenario.admin("Alice", "minecraft", "grant"), (0, None));
    assert_eq!(scenario.admin("Alice", "minecraft", "revoke"), (0, None));
    assert_eq!(scenario.admin("Alice", "minecraft", "revoke"), (1, None));
    assert_eq!(scenario.admin("Mallory", "minecraft", "deny"), (2, None));
}

#[tokio::test(flavor = "multi_thread")]
async fn malformed_requests_are_refused_with_a_reason() {
    let scenario = AdminScenario::start().await;

    let (code, error) = scenario.admin("Alice", "minecraft", "promote");
    assert_eq!(code, -1);
    assert!(error.unwrap().contains("promote"));

    let (code, error) = scenario.admin("Alice", "hytale-classic", "grant");
    assert_eq!(code, -1);
    assert!(error.unwrap().contains("hytale-classic"));

    let (code, error) = scenario.server.admin("not json");
    assert_eq!(code, -1);
    assert!(error.is_some());
}
