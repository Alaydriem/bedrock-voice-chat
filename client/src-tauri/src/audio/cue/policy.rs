use crate::audio::cue::Cue;
use common::structs::audio::AudioDeviceType;
use common::structs::channel::ChannelEvents;

/// Which cue a mute change earns, if any.
///
/// Separate from the manager that plays it because the manager needs an `AppHandle` and
/// this decision does not. The rules here are the ones that go wrong quietly — a cue on a
/// write that changed nothing fires once a second forever, and nothing about the sound says
/// which surface asked for it.
pub struct CuePolicy;

impl CuePolicy {
    pub fn for_change(device: &AudioDeviceType, previous: bool, next: bool) -> Option<Cue> {
        if previous == next {
            return None;
        }

        Some(match (device, next) {
            (AudioDeviceType::InputDevice, true) => Cue::Mute,
            (AudioDeviceType::InputDevice, false) => Cue::Unmute,
            (AudioDeviceType::OutputDevice, true) => Cue::Deafen,
            (AudioDeviceType::OutputDevice, false) => Cue::Undeafen,
        })
    }

    /// Which cue a channel event earns, if any.
    ///
    /// Every client hears every channel event, including the ones for groups it is not in,
    /// because the groups pane is built from them. Only the local player's own move is
    /// announced; `is_self` is the whole of that decision and the caller supplies it.
    ///
    /// Create, delete and rename are not membership changes. A delete in particular names
    /// the player who deleted the group rather than each member who lost a seat, so a cue
    /// on it would announce somebody else's action to everyone who happened to be there.
    pub fn for_channel_event(event: &ChannelEvents, is_self: bool) -> Option<Cue> {
        if !is_self {
            return None;
        }

        match event {
            ChannelEvents::Join => Some(Cue::GroupJoin),
            ChannelEvents::Leave => Some(Cue::GroupLeave),
            ChannelEvents::Create | ChannelEvents::Delete | ChannelEvents::Rename => None,
        }
    }
}
