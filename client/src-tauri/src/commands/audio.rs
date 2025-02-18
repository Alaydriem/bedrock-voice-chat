use common::structs::audio::{AudioDevice, AudioDeviceType};
use std::sync::Mutex;
use std::collections::HashMap;
use tauri::{AppHandle, Emitter, State};
use tauri_plugin_store::StoreExt;

use crate::{
    audio::events::{ChangeAudioDeviceEvent, StopAudioDeviceEvent},
    structs::app_state::AppState, AudioStreamManager,
};
use log::{info, error};

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
pub(crate) async fn change_audio_device(
    device: AudioDevice,
    app: AppHandle,
    state: State<'_, tauri::async_runtime::Mutex<AppState>>,
    asm: State<'_, tauri::async_runtime::Mutex<AudioStreamManager>> // tauri::async_runtime::Mutex to fix lock issue
) -> Result<(), ()> {
    let mut state = state.lock().await;
    _ = stop_audio_device(device.io.clone(), app.clone());
    _ = update_current_player(app.clone(), asm.clone());
    state.change_audio_device(&device);

    _ = app.emit("change-audio-device", ChangeAudioDeviceEvent { device: device.clone() });
    let mut asm = asm.lock().await;
    asm.init(device.clone());
    _ = asm.restart(&device.clone().io).await;   

    Ok(())
}

// Maps the current player information to the Audio Output Stream
#[tauri::command]
pub(crate) async fn update_current_player(
    app: AppHandle,
    asm: State<'_, tauri::async_runtime::Mutex<AudioStreamManager>>
) -> Result<(), ()>{
    let mut asm = asm.lock().await;
    match app.store("store.json") {
        Ok(store) => match store.get("current_player"){
            Some(value) => match value.get("value") {
                Some(value) => {
                    let current_player = value.to_string();
                    _ = asm.metadata(
                        String::from("current_player"),
                        current_player,
                        &AudioDeviceType::OutputDevice
                    );
                },
                None => return Err(())
            },
            None => return Err(())
        },                    
        Err(_) => return Err(())    
    };

    Ok(())
}

/// Stops the audio stream for a given device
/// This will trigger a "stop-audio-device" event
#[tauri::command]
pub(crate) async fn stop_audio_device(
    device: AudioDeviceType,
    app: AppHandle
) {
    _ = app.emit("stop-audio-device", StopAudioDeviceEvent { device });
}

/// Returns a list of audio devices
#[tauri::command]
pub(crate) async fn get_devices() -> Result<HashMap<String, Vec<AudioDevice>>, ()> {
    return crate::audio::device::get_devices();
}
