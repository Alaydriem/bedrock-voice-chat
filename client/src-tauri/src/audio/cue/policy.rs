use crate::audio::cue::Cue;
use common::structs::audio::AudioDeviceType;
use common::structs::channel::ChannelEvents;
use std::time::{Duration, Instant};

/// Which cue a mute change earns, if any.
///
/// Separate from the manager that plays it because the manager needs an `AppHandle` and
/// this decision does not. The rules here are the ones that go wrong quietly — a cue on a
/// write that changed nothing fires once a second forever, and nothing about the sound says
/// which surface asked for it.
pub struct CuePolicy;

impl CuePolicy {
    pub const JOIN_REQUEST_INTERVAL: Duration = Duration::from_secs(30);

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
    /// Whether a new join request card earns the join-request cue. At most one per
    /// [`Self::JOIN_REQUEST_INTERVAL`]: one sound tells the member to look at the dashboard, and
    /// a server raising many requests at once must not turn that into a burst.
    pub fn for_join_request(last_played: Option<Instant>, now: Instant) -> Option<Cue> {
        let due = last_played
            .is_none_or(|last| now.duration_since(last) >= Self::JOIN_REQUEST_INTERVAL);
        due.then_some(Cue::JoinRequest)
    }

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
