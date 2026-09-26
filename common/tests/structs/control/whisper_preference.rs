use common::consts::audio::{JUKEBOX_CONTROL_TARGET, WHISPER_CONTROL_TARGET};
use common::structs::control::{PlayerPreference, WhisperPreference};

#[test]
fn an_opted_in_preference_reads_as_enabled() {
    let preference = WhisperPreference::for_owner("minecraft:Alice", true);

    assert_eq!(preference.target, WHISPER_CONTROL_TARGET);
    assert!(WhisperPreference::is_enabled(&preference));
}

#[test]
fn an_opted_out_preference_reads_as_disabled() {
    let preference = WhisperPreference::for_owner("minecraft:Alice", false);

    assert!(!WhisperPreference::is_enabled(&preference));
}

// The plane is shared. A muted jukebox is `muted: true` too, and reading it as an opt-in would
// shrink the range of every player who silenced their music.
#[test]
fn another_target_never_reads_as_an_opt_in() {
    let jukebox = PlayerPreference {
        owner: "minecraft:Alice".to_string(),
        target: JUKEBOX_CONTROL_TARGET.to_string(),
        volume: 1.0,
        muted: true,
    };

    assert!(!WhisperPreference::is_enabled(&jukebox));
}
