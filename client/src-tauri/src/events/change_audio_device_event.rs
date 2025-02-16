use common::structs::audio::AudioDevice;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ChangeAudioDeviceEvent {
    pub device: AudioDevice,
}
