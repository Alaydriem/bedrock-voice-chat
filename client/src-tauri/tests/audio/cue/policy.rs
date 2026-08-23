use bvc_client_lib::audio::{Cue, CuePolicy};
use common::structs::audio::AudioDeviceType;

/// Mute is driven from a dozen surfaces, several of which set a value rather than flip one:
/// the 1 Hz self-state poll, an idempotent Stream Deck `set_mute`, the in-game panel
/// resyncing. A cue on every write would fire a tone once a second forever.
#[test]
fn a_change_that_changes_nothing_is_silent() {
    for device in [AudioDeviceType::InputDevice, AudioDeviceType::OutputDevice] {
        assert_eq!(CuePolicy::for_change(&device, true, true), None);
        assert_eq!(CuePolicy::for_change(&device, false, false), None);
    }
}

#[test]
fn the_microphone_gets_the_mute_pair() {
    assert_eq!(
        CuePolicy::for_change(&AudioDeviceType::InputDevice, false, true),
        Some(Cue::Mute)
    );
    assert_eq!(
        CuePolicy::for_change(&AudioDeviceType::InputDevice, true, false),
        Some(Cue::Unmute)
    );
}

/// Muting the output device is what deafening is. A caller reaching that flag directly —
/// the keybind, a WebSocket controller — must produce the deafen cue, not the mute one, or
/// the same state change sounds different depending on which button caused it.
#[test]
fn the_output_device_gets_the_deafen_pair() {
    assert_eq!(
        CuePolicy::for_change(&AudioDeviceType::OutputDevice, false, true),
        Some(Cue::Deafen)
    );
    assert_eq!(
        CuePolicy::for_change(&AudioDeviceType::OutputDevice, true, false),
        Some(Cue::Undeafen)
    );
}
