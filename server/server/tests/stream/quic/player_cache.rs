use bvc_server_lib::stream::quic::CacheManager;
use common::game_data::Dimension;
use common::structs::packet::{
    PacketType, PlayerDataPacket, QuicNetworkPacket, QuicNetworkPacketData,
};
use common::{Coordinate, MinecraftPlayer, Orientation, PlayerEnum};

fn player(name: &str, world: Option<&str>) -> PlayerEnum {
    PlayerEnum::Minecraft(MinecraftPlayer {
        name: name.to_string(),
        coordinates: Coordinate {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        orientation: Orientation { x: 0.0, y: 0.0 },
        dimension: Dimension::Overworld,
        deafen: false,
        spectator: false,
        world_uuid: None,
        alternative_identity: None,
        player_uuid: None,
        relay_world_uuid: world.map(str::to_string),
        bridged_voice: false,
    })
}

// Populated the way the server populates it: a server-injected PlayerData packet,
// which carries no sender device and so is accepted as authoritative.
async fn caches_holding(players: Vec<PlayerEnum>) -> CacheManager {
    let caches = CacheManager::new();

    caches
        .process_packet(QuicNetworkPacket {
            packet_type: PacketType::PlayerData,
            data: QuicNetworkPacketData::PlayerData(PlayerDataPacket { players }),
            ..Default::default()
        })
        .await
        .expect("player data is accepted");

    caches
}

#[tokio::test]
async fn counts_players_per_world_and_sorts_by_name() {
    let caches = caches_holding(vec![
        player("Alice", Some("overworld")),
        player("Bob", Some("overworld")),
        player("Cara", Some("nether")),
    ])
    .await;

    assert_eq!(
        caches.relay_world_populations(),
        vec![("nether".to_string(), 1), ("overworld".to_string(), 2)]
    );
}

// A player with no relay world is not in one. Counting them under a placeholder
// would make an operator reading the listing believe a world exists that no peer
// can ever be granted.
#[tokio::test]
async fn omits_players_with_no_relay_world() {
    let caches = caches_holding(vec![player("Alice", None)]).await;

    assert!(caches.relay_world_populations().is_empty());
}
