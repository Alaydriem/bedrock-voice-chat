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

    assert!(
        volume < 0.001,
        "Expected silence beyond falloff, got {}",
        volume
    );
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

fn whispered(x: f32, config: &SpatialAudioConfig) -> (f32, f32) {
    let result = SpatialCalculator::gains(
        &at(x, 0.0, 0.0),
        true,
        &origin(),
        &facing(0.0),
        Game::Minecraft,
        config,
    );

    (result.pan, result.volume)
}

#[test]
fn a_whisper_beside_the_listener_is_full_volume() {
    let (_, volume) = whispered(0.5, &SpatialAudioConfig::default());

    assert_eq!(volume, 1.0);
}

// The normal curve, compressed into the whisper range: quieter the further away, rather than
// flat right up to a cut.
#[test]
fn a_whisper_fades_inside_its_range() {
    let config = SpatialAudioConfig::default();
    let edge = config.whisper_edge();
    let (_, near) = whispered(edge * 0.6, &config);
    let (_, far) = whispered(edge * 0.9, &config);

    assert!(near < 1.0, "near {near}");
    assert!(far < near, "far {far} near {near}");
    assert!(far > 0.0, "far {far}");
}

// The server stops sending at the edge, so the curve has to be silent there already, or the
// listener hears the cut the change exists to remove.
#[test]
fn a_whisper_is_silent_at_and_past_its_edge() {
    let config = SpatialAudioConfig::default();
    let edge = config.whisper_edge();

    let (_, at_edge) = whispered(edge, &config);
    let (_, past_edge) = whispered(edge + 0.5, &config);

    assert!(at_edge < 0.001, "at edge {at_edge}");
    assert_eq!(past_edge, 0.0);
}

#[test]
fn a_whisper_keeps_its_direction() {
    let config = SpatialAudioConfig::default();
    let (pan, _) = whispered(config.whisper_edge() * 0.5, &config);

    assert!(pan > 0.5, "Expected a whisper to the east to pan left, got {pan}");
}

// An operator can set the distance to zero. That must mean nobody hears a whisper, and never a
// division that produces NaN on the audio path.
#[test]
fn a_zero_whisper_distance_is_silent() {
    let config = SpatialAudioConfig {
        whisper_distance: 0.0,
        ..SpatialAudioConfig::default()
    };
    let (pan, volume) = whispered(0.0, &config);

    assert_eq!(volume, 0.0);
    assert_eq!(pan, 0.0);
}
