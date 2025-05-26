use common::structs::audio::{AudioDevice, AudioDeviceType};
use std::collections::HashMap;
use tauri::{AppHandle, State};
use tauri_plugin_store::StoreExt;

use crate::{
    structs::app_state::AppState, AudioStreamManager,
};
use tauri::async_runtime::Mutex;

/// Returns the active audio device for the given device type
#[tauri::command]
pub(crate) async fn get_audio_device(
    io: AudioDeviceType,
    state: State<'_, Mutex<AppState>>,
) -> Result<AudioDevice, ()> {
    let state = state.lock().await;

    return Ok(state.get_audio_device(io));
}

/// Changes the audio device for the selected audio device type
/// This will emit a "stop-audio-device" event, followed by a "change-audio-device" event
/// Which will result in the specific stream being stopped, and a new one being started
#[tauri::command]
pub(crate) async fn change_audio_device(
    device: AudioDevice,
    app: AppHandle,
    state: State<'_, Mutex<AppState>>,
    asm: State<'_, Mutex<AudioStreamManager>>
) -> Result<(), ()> {
    let mut state = state.lock().await;
    _ = update_current_player(app.clone(), asm.clone());
    state.change_audio_device(device.clone());

    let mut asm = asm.lock().await;
    asm.init(device.clone());
    _ = asm.restart(device.clone().io).await;   

    Ok(())
}

// Maps the current player information to the Audio Output Stream
#[tauri::command]
pub(crate) async fn update_current_player(
    app: AppHandle,
    asm: State<'_, Mutex<AudioStreamManager>>
) -> Result<(), ()>{
    let mut asm = asm.lock().await;
    match app.store("store.json") {
        Ok(store) => match store.get("current_player") {
            Some(value) => match value.as_str() {
                Some(value) => {
                    let current_player = value.to_string();
                    _ = asm.metadata(
                        String::from("current_player"),
                        current_player,
                        &AudioDeviceType::OutputDevice
                    ).await;
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
    asm: State<'_, Mutex<AudioStreamManager>>
) -> Result<(), ()> {
    let mut asm = asm.lock().await;

    _ = asm.stop(device).await;
    return Ok(());
}

/// Returns a list of audio devices
#[tauri::command]
pub(crate) async fn get_devices() -> Result<HashMap<String, Vec<AudioDevice>>, ()> {
    return crate::audio::device::get_devices();
}

#[tauri::command]
pub(crate) async fn mute(
    device: AudioDeviceType,
    asm: State<'_, Mutex<AudioStreamManager>>,
) -> Result<(), ()>{
    let mut asm = asm.lock().await;
    _ = asm.mute(&device).await;

    Ok(())
}