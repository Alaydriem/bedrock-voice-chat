use ts_rs::TS;
use serde::{ Deserialize, Serialize };

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "./../client/src/js/bindings/")]
pub enum AudioDeviceType {
    InputDevice,
    OutputDevice,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "./../client/src/js/bindings/")]
pub struct AudioDevice {
    pub io: AudioDeviceType,
    pub name: String,
}
