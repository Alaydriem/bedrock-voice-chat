use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::device::AudioDeviceType;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub enum MuteEvent {
    #[serde(rename = "mute:input")]
    Input,
    #[serde(rename = "mute:output")]
    Output,
}

impl std::fmt::Display for MuteEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MuteEvent::Input => write!(f, "mute:input"),
            MuteEvent::Output => write!(f, "mute:output"),
        }
    }
}

impl From<AudioDeviceType> for MuteEvent {
    fn from(device: AudioDeviceType) -> Self {
        match device {
            AudioDeviceType::InputDevice => MuteEvent::Input,
            AudioDeviceType::OutputDevice => MuteEvent::Output,
        }
    }
}

impl From<&AudioDeviceType> for MuteEvent {
    fn from(device: &AudioDeviceType) -> Self {
        match device {
            AudioDeviceType::InputDevice => MuteEvent::Input,
            AudioDeviceType::OutputDevice => MuteEvent::Output,
        }
    }
}
