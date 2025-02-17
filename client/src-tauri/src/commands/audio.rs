use common::structs::audio::{AudioDevice, AudioDeviceType};
use std::sync::Mutex;
use std::collections::HashMap;
use tauri::{AppHandle, Emitter, State};

use crate::{
    audio::events::{ChangeAudioDeviceEvent, StopAudioDeviceEvent},
    structs::app_state::AppState,
};
use log::error;

/// Returns the active audio device for the given device type
#[tauri::command]
pub(crate) async fn get_audio_device(
    io: AudioDeviceType,
    state: State<'_, Mutex<AppState>>,
) -> Result<AudioDevice, ()> {
    match state.lock() {
        Ok(state) => Ok(state.get_audio_device(io)),
        Err(e) => {
            error!("Failed to get audio device {}: {}", io.to_string(), e);
            Err(())
        }
    }
}

/// Changes the audio device for the selected audio device type
/// This will emit a "stop-audio-device" event, followed by a "change-audio-device" event
/// Which will result in the specific stream being stopped, and a new one being started
#[tauri::command]
pub(crate) fn change_audio_device(
    device: AudioDevice,
    app: AppHandle,
    state: State<'_, Mutex<AppState>>,
) {
    match state.lock() {
        Ok(mut state) => {
            state.change_audio_device(&device);
            _ = app.emit(
                "stop-audio-device",
                StopAudioDeviceEvent {
                    device: device.io.clone(),
                },
            );
            _ = app.emit("change-audio-device", ChangeAudioDeviceEvent { device });
        }
        Err(e) => error!("Failed to access AppState in `change-audio-device` {:?}", e),
    };
}

/// Stops the audio stream for a given device
/// This will trigger a "stop-audio-device" event
#[tauri::command]
pub(crate) fn stop_audio_device(
    device: AudioDeviceType,
    app: AppHandle
) {
    _ = app.emit("stop-audio-device", StopAudioDeviceEvent { device });
}

/// Returns a list of audio devices
#[tauri::command]
pub(crate) fn get_devices() -> Result<HashMap<String, Vec<AudioDevice>>, ()> {
    return crate::audio::device::get_devices();
}
