use std::time::Duration;

use common::structs::position::PresenceKind;

use crate::harness::server::EmbeddedServer;

/// The position feed, end to end: positions in over the mod's own path, a snapshot out over
/// the WebSocket a client would open.
///
/// ## What this proves
///
/// Everything between the ingest and the socket, which no other test covers and none of it
/// is verifiable by inspection:
///
///   1. `bvc_update_positions` reaches the player cache
///   2. the shared pass picks the cache up on its next tick and buckets it
///   3. a ticket redeems, and the subprotocol handshake the browser API forces is accepted
///   4. the observer is found in the index and their neighbours are turned into bearings
///      and distances relative to *them*
///
/// ## Why the assertions are shaped this way
///
/// Distance is asserted with a tolerance because it is rounded to whole blocks server-side,
/// and bearing is asserted only as in-range because the fixture faces yaw 0 and the exact
/// value is the trigonometry's business rather than this test's.
///
/// `presence` is `Game` for both: `update_positions` puts players in the world, and neither
/// of them has a QUIC voice connection. That is the case worth pinning — a player who is
/// standing right there and cannot hear you is the confusion this field exists to remove,
/// and it is also the case a test that only ever ran with voice clients would never see.
///
/// Requires the server cdylib to be built first:
/// `cargo build -p bedrock-voice-chat-server` in the `server/` workspace.
#[tokio::test(flavor = "multi_thread")]
async fn streams_relative_positions_to_an_observer() {
    let data_dir = tempfile::tempdir().expect("create temp data dir");

    let rocket_port = EmbeddedServer::free_port_tcp();
    let quic_port = EmbeddedServer::free_port_udp();

    let config_json = EmbeddedServer::config_json(rocket_port, quic_port, data_dir.path());
    let certs_path = data_dir.path().join("certificates");

    let lib = EmbeddedServer::load_library();
    let server =
        EmbeddedServer::start(lib, &config_json, rocket_port, quic_port, &certs_path).await;

    // Thirty blocks apart, which is inside the default 48-block voice range, so Bob belongs
    // to the near tier the roster is built from.
    server.update_positions(&[("Alice", 0.0, 64.0, 0.0), ("Bob", 30.0, 64.0, 0.0)]);

    let snapshots = server
        .position_snapshots("Alice", 1, Duration::from_secs(10))
        .await;

    let snapshot = snapshots
        .first()
        .expect("the feed produced no snapshot carrying an entry");

    assert_eq!(
        snapshot.positions.len(),
        1,
        "Alice should see Bob and not herself: {:?}",
        snapshot.positions
    );

    let bob = &snapshot.positions[0];
    assert_eq!(bob.name, "minecraft:Bob");
    assert_eq!(bob.presence, PresenceKind::Game);
    assert!(
        (29..=31).contains(&bob.distance),
        "expected about 30 blocks, got {}",
        bob.distance
    );
    assert!(bob.bearing_deg < 360);
    assert_eq!(bob.elevation, 0);
}

/// A player on the far side of the world is filtered by the same rule voice routing uses,
/// so the feed says nothing rather than reporting somebody unreachable.
#[tokio::test(flavor = "multi_thread")]
async fn a_player_beyond_scope_never_reaches_the_socket() {
    let data_dir = tempfile::tempdir().expect("create temp data dir");

    let rocket_port = EmbeddedServer::free_port_tcp();
    let quic_port = EmbeddedServer::free_port_udp();

    let config_json = EmbeddedServer::config_json(rocket_port, quic_port, data_dir.path());
    let certs_path = data_dir.path().join("certificates");

    let lib = EmbeddedServer::load_library();
    let server =
        EmbeddedServer::start(lib, &config_json, rocket_port, quic_port, &certs_path).await;

    server.update_positions(&[("Alice", 0.0, 64.0, 0.0), ("Bob", 10_000.0, 64.0, 0.0)]);

    // Two seconds is four ticks of the feed, so this is not merely "no snapshot yet".
    let snapshots = server
        .position_snapshots("Alice", 1, Duration::from_secs(2))
        .await;

    assert!(
        snapshots.is_empty(),
        "expected no entries for a player 10 000 blocks away: {snapshots:?}"
    );
}
