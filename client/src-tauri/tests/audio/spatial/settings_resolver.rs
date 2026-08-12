use bvc_client_lib::audio::recording::renderer::SettingsProvenance;
use bvc_client_lib::audio::spatial::SpatialSettingsResolver;
use common::structs::SpatialAudioConfig;

fn ranged(falloff: f32, intensity: f32) -> (SpatialAudioConfig, f32) {
    let config = SpatialAudioConfig {
        falloff_distance: falloff,
        ..SpatialAudioConfig::default()
    };

    (config, intensity)
}

#[test]
fn the_live_session_wins_when_it_has_values() {
    let chosen =
        SpatialSettingsResolver::choose(Some(ranged(64.0, 0.5)), Some(ranged(96.0, 0.9)));

    assert_eq!(chosen.config().falloff_distance, 64.0);
    assert_eq!(chosen.provenance(), SettingsProvenance::LiveSession);
}

// Exporting the morning after recording is the normal case, and there is no live session then.
// Without this the render silently runs on the compiled curve.
#[test]
fn the_last_known_values_are_used_when_there_is_no_live_session() {
    let chosen = SpatialSettingsResolver::choose(None, Some(ranged(96.0, 0.9)));

    assert_eq!(chosen.config().falloff_distance, 96.0);
    assert_eq!(chosen.provenance(), SettingsProvenance::LastKnown);
}

#[test]
fn defaults_are_the_last_resort_and_say_so() {
    let chosen = SpatialSettingsResolver::choose(None, None);

    assert_eq!(
        chosen.config().falloff_distance,
        SpatialAudioConfig::default().falloff_distance
    );
    assert_eq!(chosen.panning_intensity(), 0.8);
    assert_eq!(chosen.provenance(), SettingsProvenance::Defaults);
}

#[test]
fn the_panning_intensity_travels_with_the_config_it_came_from() {
    let chosen = SpatialSettingsResolver::choose(Some(ranged(64.0, 0.5)), None);

    assert_eq!(chosen.panning_intensity(), 0.5);
}

#[test]
fn a_last_known_value_is_not_mixed_into_a_live_one() {
    let chosen =
        SpatialSettingsResolver::choose(Some(ranged(64.0, 0.5)), Some(ranged(96.0, 0.9)));

    assert_eq!(chosen.panning_intensity(), 0.5);
}
