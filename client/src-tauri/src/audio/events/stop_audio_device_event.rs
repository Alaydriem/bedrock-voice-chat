use common::structs::audio::AudioDeviceType;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StopAudioDeviceEvent {
    pub device: AudioDeviceType,
}
