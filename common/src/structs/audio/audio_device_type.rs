use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub enum AudioDeviceType {
    InputDevice,
    OutputDevice,
}

impl AudioDeviceType {
    pub fn to_string(&self) -> String {
        match self {
            AudioDeviceType::InputDevice => "input_audio_device".to_string(),
            AudioDeviceType::OutputDevice => "output_audio_device".to_string(),
        }
    }
}
