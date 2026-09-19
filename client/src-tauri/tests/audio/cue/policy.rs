use bvc_client_lib::audio::{Cue, CuePolicy};
use common::structs::audio::AudioDeviceType;
use common::structs::channel::ChannelEvents;

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

/// Group membership changes for everybody else reach this client too — the roster in the
/// groups pane is built from them. A tone on each would turn a busy server into a chime
/// machine, so only the local player's own move is announced.
#[test]
fn another_players_membership_change_is_silent() {
    assert_eq!(CuePolicy::for_channel_event(&ChannelEvents::Join, false), None);
    assert_eq!(
        CuePolicy::for_channel_event(&ChannelEvents::Leave, false),
        None
    );
}

#[test]
fn the_local_player_gets_the_group_pair() {
    assert_eq!(
        CuePolicy::for_channel_event(&ChannelEvents::Join, true),
        Some(Cue::GroupJoin)
    );
    assert_eq!(
        CuePolicy::for_channel_event(&ChannelEvents::Leave, true),
        Some(Cue::GroupLeave)
    );
}

/// Creating a group does not join it, renaming one does not move anybody, and a delete
/// names the player who deleted it rather than each member losing their seat. None of the
/// three is a membership change, and a tone on any of them announces something that did
/// not happen to the person hearing it.
#[test]
fn events_that_are_not_membership_changes_are_silent() {
    for event in [
        ChannelEvents::Create,
        ChannelEvents::Delete,
        ChannelEvents::Rename,
    ] {
        assert_eq!(CuePolicy::for_channel_event(&event, true), None);
        assert_eq!(CuePolicy::for_channel_event(&event, false), None);
    }
}
