use serde::{ Serialize, Deserialize };
use common::structs::audio::AudioDeviceType;

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StopAudioDeviceEvent {
  pub device: AudioDeviceType
}