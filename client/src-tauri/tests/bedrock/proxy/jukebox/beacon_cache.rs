use bvc_client_lib::bedrock::JukeboxBeaconCache;
use common::structs::game::BlockCoordinate;
use common::game_data::Dimension;

#[test]
fn observe_then_resolve_returns_event_id_once() {
    let cache = JukeboxBeaconCache::new();
    let pos = BlockCoordinate::new(12, 64, -7);
    cache.observe(pos, Dimension::Overworld, "evt-abc");
    assert_eq!(
        cache
            .resolve_for_eject(pos, Dimension::Overworld)
            .as_deref(),
        Some("evt-abc")
    );
    assert!(cache.resolve_for_eject(pos, Dimension::Overworld).is_none());
}

#[test]
fn unknown_position_returns_none() {
    let cache = JukeboxBeaconCache::new();
    assert!(
        cache
            .resolve_for_eject(BlockCoordinate::new(0, 0, 0), Dimension::Overworld)
            .is_none()
    );
}

#[test]
fn same_coords_different_dimension_do_not_collide() {
    let cache = JukeboxBeaconCache::new();
    let pos = BlockCoordinate::new(100, 64, 100);
    cache.observe(pos, Dimension::Overworld, "evt-overworld");
    cache.observe(pos, Dimension::TheNether, "evt-nether");
    assert_eq!(
        cache
            .resolve_for_eject(pos, Dimension::TheNether)
            .as_deref(),
        Some("evt-nether")
    );
    assert_eq!(
        cache
            .resolve_for_eject(pos, Dimension::Overworld)
            .as_deref(),
        Some("evt-overworld")
    );
}

#[test]
fn floor_handles_negative_coords() {
    use common::structs::game::Coordinate;
    let cache = JukeboxBeaconCache::new();
    let coord = Coordinate {
        x: -0.5,
        y: -10.9,
        z: -100.1,
    };
    let key = BlockCoordinate::from(&coord);
    cache.observe(key, Dimension::Overworld, "evt-neg");
    assert_eq!(
        cache
            .resolve_for_eject(BlockCoordinate::new(-1, -11, -101), Dimension::Overworld)
            .as_deref(),
        Some("evt-neg"),
    );
}
