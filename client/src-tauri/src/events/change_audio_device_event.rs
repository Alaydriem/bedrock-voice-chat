use serde::{ Serialize, Deserialize };
use common::structs::audio::AudioDevice;

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ChangeAudioDeviceEvent {
  pub device: AudioDevice
}