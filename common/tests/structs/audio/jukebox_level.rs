use common::consts::audio::JUKEBOX_PLAYER_PREFIX;
use common::structs::audio::settings::JukeboxLevel;

fn jukebox_key() -> String {
    format!("{}60fc0bd7", JUKEBOX_PLAYER_PREFIX)
}

#[test]
fn a_fresh_level_leaves_a_jukebox_untouched() {
    let level = JukeboxLevel::new();

    let settings = level.settings_for(&jukebox_key());

    assert_eq!(settings.gain, 1.0);
    assert!(!settings.muted);
}

#[test]
fn a_jukebox_takes_the_configured_gain() {
    let level = JukeboxLevel::new();
    level.set_gain(0.6);

    assert_eq!(level.settings_for(&jukebox_key()).gain, 0.6);
}

// Two playbacks are two sink keys, and each resolves on its own. Asking twice with different
// keys is how the caller's per-sink loop uses this, so it is how it is tested.
#[test]
fn every_concurrent_jukebox_resolves_independently_to_the_same_opinion() {
    let level = JukeboxLevel::new();
    level.set_gain(0.4);

    let first = level.settings_for(&format!("{}aaaaaaaa", JUKEBOX_PLAYER_PREFIX));
    let second = level.settings_for(&format!("{}bbbbbbbb", JUKEBOX_PLAYER_PREFIX));

    assert_eq!(first.gain, 0.4);
    assert_eq!(second.gain, 0.4);
}

// Channel API audio is synthetic too and reaches the caller's same match arm. A music control
// that silenced an announcement would be a different feature.
#[test]
fn a_speaker_that_is_not_a_jukebox_is_untouched_even_when_jukeboxes_are_muted() {
    let level = JukeboxLevel::new();
    level.set_muted(true);
    level.set_gain(0.0);

    let settings = level.settings_for("minecraft:Alaydriem");

    assert_eq!(settings.gain, 1.0);
    assert!(!settings.muted);
}

// The two are separate controls on every surface, so unmuting has to come back to the level
// that was set rather than to unity.
#[test]
fn muting_keeps_the_gain_that_was_set() {
    let level = JukeboxLevel::new();
    level.set_gain(0.35);

    level.set_muted(true);
    assert!(level.settings_for(&jukebox_key()).muted);

    level.set_muted(false);
    assert_eq!(level.settings_for(&jukebox_key()).gain, 0.35);
}

#[test]
fn a_gain_beyond_the_range_is_clamped() {
    let level = JukeboxLevel::new();

    level.set_gain(9.0);
    assert_eq!(level.gain(), JukeboxLevel::MAX_GAIN);

    level.set_gain(-2.0);
    assert_eq!(level.gain(), 0.0);
}
