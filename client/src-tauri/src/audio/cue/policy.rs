use crate::audio::cue::Cue;
use common::structs::audio::AudioDeviceType;

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
}
