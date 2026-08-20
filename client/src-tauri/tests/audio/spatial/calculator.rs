use bvc_client_lib::audio::spatial::SpatialCalculator;
use common::structs::SpatialAudioConfig;
use common::{Coordinate, Game, Orientation};

fn at(x: f32, y: f32, z: f32) -> Coordinate {
    Coordinate { x, y, z }
}

fn origin() -> Coordinate {
    at(0.0, 0.0, 0.0)
}

// `Orientation` carries pitch in `x` and yaw in `y`, and nothing else. Only yaw is read.
fn facing(yaw: f32) -> Orientation {
    Orientation { x: 0.0, y: yaw }
}

fn heard(emitter: Coordinate, yaw: f32, game: Game) -> (f32, f32) {
    let result = SpatialCalculator::gains(
        &emitter,
        false,
        &origin(),
        &facing(yaw),
        game,
        &SpatialAudioConfig::default(),
    );

    (result.pan, result.volume)
}

// Minecraft yaw 0 faces south (+Z), so the listener's left is east (+X).
#[test]
fn minecraft_facing_south_emitter_east_pans_left() {
    let (pan, _) = heard(at(20.0, 0.0, 0.0), 0.0, Game::Minecraft);

    assert!(pan > 0.5, "Expected positive pan (left), got {}", pan);
}

#[test]
fn minecraft_facing_south_emitter_west_pans_right() {
    let (pan, _) = heard(at(-20.0, 0.0, 0.0), 0.0, Game::Minecraft);

    assert!(pan < -0.5, "Expected negative pan (right), got {}", pan);
}

#[test]
fn minecraft_facing_south_emitter_ahead_centered() {
    let (pan, _) = heard(at(0.0, 0.0, 20.0), 0.0, Game::Minecraft);

    assert!(pan.abs() < 0.01, "Expected centered pan, got {}", pan);
}

// Directly behind is as centred as directly ahead. The engine has no front-back cue.
#[test]
fn minecraft_facing_south_emitter_behind_centered() {
    let (pan, _) = heard(at(0.0, 0.0, -20.0), 0.0, Game::Minecraft);

    assert!(pan.abs() < 0.01, "Expected centered pan, got {}", pan);
}

// Yaw 90 faces west (-X), which puts south (+Z) on the listener's left.
#[test]
fn minecraft_facing_west_emitter_south_pans_left() {
    let (pan, _) = heard(at(0.0, 0.0, 20.0), 90.0, Game::Minecraft);

    assert!(pan > 0.5, "Expected positive pan (left), got {}", pan);
}

#[test]
fn close_range_suppresses_panning() {
    let config = SpatialAudioConfig::default();
    let (pan, volume) = heard(
        at(config.panning_start - 3.0, 0.0, 0.0),
        0.0,
        Game::Minecraft,
    );

    assert!(
        pan.abs() < 0.01,
        "Expected suppressed pan at close range, got {}",
        pan
    );
    assert!(
        (volume - 1.0).abs() < 0.01,
        "Expected full volume at close range, got {}",
        volume
    );
}

#[test]
fn mid_range_ramps_panning() {
    let (pan, _) = heard(at(10.0, 0.0, 0.0), 0.0, Game::Minecraft);

    assert!(pan > 0.0 && pan < 1.0, "Expected partial pan, got {}", pan);
}

#[test]
fn inside_the_close_threshold_is_full_volume() {
    let config = SpatialAudioConfig::default();
    let (_, volume) = heard(
        at(config.close_threshold - 1.0, 0.0, 0.0),
        0.0,
        Game::Minecraft,
    );

    assert_eq!(volume, 1.0);
}

#[test]
fn beyond_falloff_is_silent() {
    let config = SpatialAudioConfig::default();
    let (_, volume) = heard(
        at(config.falloff_distance + 2.0, 0.0, 0.0),
        0.0,
        Game::Minecraft,
    );

    assert!(volume < 0.001, "Expected silence beyond falloff, got {}", volume);
}

#[test]
fn volume_attenuates_with_distance() {
    let (_, near) = heard(at(0.0, 0.0, 30.0), 0.0, Game::Minecraft);
    let (_, far) = heard(at(0.0, 0.0, 40.0), 0.0, Game::Minecraft);

    assert!(
        near > far,
        "Near volume {} should exceed far volume {}",
        near,
        far
    );
    assert!(near > 0.0 && near < 1.0);
    assert!(far > 0.0);
}

// Past the steepen point the dB curve is multiplied down toward zero at the falloff edge, so
// the last stretch falls faster than the curve alone would.
#[test]
fn attenuation_steepens_past_the_steepen_start() {
    let config = SpatialAudioConfig::default();
    let (_, before) = heard(
        at(config.steepen_start - 4.0, 0.0, 0.0),
        0.0,
        Game::Minecraft,
    );
    let (_, after) = heard(
        at(config.steepen_start + 4.0, 0.0, 0.0),
        0.0,
        Game::Minecraft,
    );
    let (_, edge) = heard(at(config.falloff_distance, 0.0, 0.0), 0.0, Game::Minecraft);

    assert!(after < before);
    assert!(edge < 0.01);
}

// Vertical separation counts toward distance but never toward direction.
#[test]
fn height_changes_volume_but_not_pan() {
    let (level_pan, level_volume) = heard(at(30.0, 0.0, 0.0), 0.0, Game::Minecraft);
    let (above_pan, above_volume) = heard(at(30.0, 20.0, 0.0), 0.0, Game::Minecraft);

    assert!(above_volume < level_volume);
    assert!(above_pan.abs() < level_pan.abs());
}

#[test]
fn an_emitter_on_top_of_the_listener_is_centred() {
    let (pan, _) = heard(origin(), 0.0, Game::Minecraft);

    assert_eq!(pan, 0.0);
}

// The server enforces the deafen distance, so a frame that arrived is played flat rather than
// being attenuated a second time on this side.
#[test]
fn deafen_plays_at_full_volume() {
    let result = SpatialCalculator::gains(
        &at(2.0, 0.0, 0.0),
        true,
        &origin(),
        &facing(0.0),
        Game::Minecraft,
        &SpatialAudioConfig::default(),
    );

    assert!((result.volume - 1.0).abs() < 0.01);
    assert!(result.pan.abs() < 0.01);
}

// Deafen wins over the falloff cut: an emitter past the edge is still audible when the packet
// arrived because the emitter was deafened.
#[test]
fn deafen_is_checked_before_the_falloff_cut() {
    let config = SpatialAudioConfig::default();
    let result = SpatialCalculator::gains(
        &at(config.falloff_distance + 20.0, 0.0, 0.0),
        true,
        &origin(),
        &facing(0.0),
        Game::Minecraft,
        &config,
    );

    assert_eq!(result.volume, 1.0);
}
