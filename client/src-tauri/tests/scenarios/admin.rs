use crate::harness::server::EmbeddedServer;

/// Boot the real BVC server in-process and drive `bvc_admin`. The override is observed
/// through revoke's answer: revoke reports 0 only when an override existed.
///
/// Requires the server cdylib: `mise run test-client` builds it first.
#[tokio::test(flavor = "multi_thread")]
async fn grant_revoke_and_deny_round_trip_through_one_override() {
    let data_dir = tempfile::tempdir().expect("create temp data dir");
    let rocket_port = EmbeddedServer::free_port_tcp();
    let quic_port = EmbeddedServer::free_port_udp();
    let config_json = EmbeddedServer::config_json(rocket_port, quic_port, data_dir.path());
    let certs_path = data_dir.path().join("certificates");
    let lib = EmbeddedServer::load_library();
    let server =
        EmbeddedServer::start(lib, &config_json, rocket_port, quic_port, &certs_path).await;

    assert_eq!(server.admin("Alice", "minecraft", "grant"), (0, None));
    assert_eq!(server.admin("Alice", "minecraft", "grant"), (0, None));
    assert_eq!(server.admin("Alice", "minecraft", "revoke"), (0, None));
    assert_eq!(server.admin("Alice", "minecraft", "revoke"), (1, None));
    assert_eq!(server.admin("Alice", "minecraft", "deny"), (0, None));
    assert_eq!(server.admin("Alice", "minecraft", "revoke"), (0, None));
}

/// The gamertag is used verbatim: spaces survive, and a case difference names a different
/// player. Revoke never creates a player, so a never-seen gamertag has nothing to revoke.
#[tokio::test(flavor = "multi_thread")]
async fn gamertags_are_verbatim_and_revoke_never_creates() {
    let data_dir = tempfile::tempdir().expect("create temp data dir");
    let rocket_port = EmbeddedServer::free_port_tcp();
    let quic_port = EmbeddedServer::free_port_udp();
    let config_json = EmbeddedServer::config_json(rocket_port, quic_port, data_dir.path());
    let certs_path = data_dir.path().join("certificates");
    let lib = EmbeddedServer::load_library();
    let server =
        EmbeddedServer::start(lib, &config_json, rocket_port, quic_port, &certs_path).await;

    assert_eq!(server.admin("Some Name", "minecraft", "grant"), (0, None));
    assert_eq!(server.admin("Some Name", "minecraft", "revoke"), (0, None));

    assert_eq!(server.admin("Alice", "minecraft", "grant"), (0, None));
    assert_eq!(server.admin("alice", "minecraft", "revoke"), (2, None));
    assert_eq!(server.admin("Alice", "minecraft", "revoke"), (0, None));

    assert_eq!(server.admin("NeverSeen", "minecraft", "revoke"), (2, None));
    assert_eq!(server.admin("NeverSeen", "minecraft", "revoke"), (2, None));
}

/// A player row is what lets a gamertag sign in to BVC, so a deny must never create one:
/// denying admin to someone who could not sign in would hand them a login instead.
#[tokio::test(flavor = "multi_thread")]
async fn deny_never_creates_a_player() {
    let data_dir = tempfile::tempdir().expect("create temp data dir");
    let rocket_port = EmbeddedServer::free_port_tcp();
    let quic_port = EmbeddedServer::free_port_udp();
    let config_json = EmbeddedServer::config_json(rocket_port, quic_port, data_dir.path());
    let certs_path = data_dir.path().join("certificates");
    let lib = EmbeddedServer::load_library();
    let server =
        EmbeddedServer::start(lib, &config_json, rocket_port, quic_port, &certs_path).await;

    assert_eq!(server.admin("Mallory", "minecraft", "deny"), (2, None));
    assert_eq!(server.admin("Mallory", "minecraft", "deny"), (2, None));
    // Still unknown: the deny wrote no row for revoke to find.
    assert_eq!(server.admin("Mallory", "minecraft", "revoke"), (2, None));
}

#[tokio::test(flavor = "multi_thread")]
async fn invalid_input_is_refused_with_a_reason() {
    let data_dir = tempfile::tempdir().expect("create temp data dir");
    let rocket_port = EmbeddedServer::free_port_tcp();
    let quic_port = EmbeddedServer::free_port_udp();
    let config_json = EmbeddedServer::config_json(rocket_port, quic_port, data_dir.path());
    let certs_path = data_dir.path().join("certificates");
    let lib = EmbeddedServer::load_library();
    let server =
        EmbeddedServer::start(lib, &config_json, rocket_port, quic_port, &certs_path).await;

    let (code, error) = server.admin("Alice", "minecraft", "promote");
    assert_eq!(code, -1);
    assert!(error.unwrap().contains("promote"));

    let (code, error) = server.admin("Alice", "hytale-classic", "grant");
    assert_eq!(code, -1);
    assert!(error.unwrap().contains("hytale-classic"));

    let (code, _) = server.admin("   ", "minecraft", "grant");
    assert_eq!(code, -1);

    let (code, _) = server.admin(&"x".repeat(65), "minecraft", "grant");
    assert_eq!(code, -1);

    // A refused grant wrote nothing, so Alice is still not a player.
    assert_eq!(server.admin(&"x".repeat(65), "minecraft", "revoke").0, -1);
    assert_eq!(server.admin("Alice", "minecraft", "revoke"), (2, None));
}
