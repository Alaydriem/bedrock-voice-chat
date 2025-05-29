use common::structs::audio::{AudioDevice, AudioDeviceType};
use std::{collections::HashMap, time::Duration};
use tauri::{AppHandle, Manager, State};
use tauri_plugin_store::StoreExt;
use flume::{Receiver, Sender};
use crate::audio::AudioPacket;
use crate::network::NetworkPacket;
use std::sync::Arc;
use crate::{
    structs::app_state::AppState, AudioStreamManager,
};
use log::info;
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

/// Sets the audio device for a given device type in the application store state
#[tauri::command]
pub(crate) async fn set_audio_device(
    device: AudioDevice,
    app: AppHandle,
    state: State<'_, Mutex<AppState>>,
    asm: State<'_, Mutex<AudioStreamManager>>
) -> Result<(), ()> {
    let mut state = state.lock().await;
    _ = update_current_player(app.clone(), asm.clone());
    state.change_audio_device(device.clone());
    Ok(())
}

/// Hard resets the audio stream manager with the new devices
#[tauri::command]
pub(crate) async fn change_audio_device(
    app: AppHandle,
    state: State<'_, Mutex<AppState>>,
    asm: State<'_, Mutex<AudioStreamManager>>
) -> Result<(), ()> {
    let state = state.lock().await;
    
    // Reset the AudioStreamManager
    _ = reset_asm(app.clone(), asm.clone()).await;

    // Fetch the new Audio Stream Manager instance
    let asm = app.state::<Mutex<AudioStreamManager>>();
    let mut asm_active = asm.lock().await;
 
    // Reinitialize the input and output devices
    let input_device = state.get_audio_device(AudioDeviceType::InputDevice);
    let output_device = state.get_audio_device(AudioDeviceType::OutputDevice);

    asm_active.init(input_device.clone());
    _ = asm_active.start(input_device.clone().io).await;
    asm_active.init(output_device.clone());
    _ = asm_active.start(output_device.clone().io).await;

    drop(asm_active);
    let asm = app.state::<Mutex<AudioStreamManager>>();

    _ = update_current_player(app.clone(), asm.clone());

    Ok(())
}

#[tauri::command]
pub(crate) async fn update_stream_metadata(
    key: String,
    value: String,
    device: AudioDeviceType,
    asm: State<'_, Mutex<AudioStreamManager>>
) -> Result<(), ()>{
    let mut asm = asm.lock().await;
    _ = asm.metadata(
        key,
        value,
        &device
    ).await;

    Ok(())
}

#[tauri::command]
pub(crate) async fn reset_asm(
    handle: AppHandle,
    asm: State<'_, Mutex<AudioStreamManager>>
) -> Result<(), ()>{
    let mut asm = asm.lock().await;
    _ = asm.stop(AudioDeviceType::OutputDevice).await;
    _ = asm.stop(AudioDeviceType::InputDevice).await;

    _ = tokio::time::sleep(Duration::from_millis(100)).await;

    handle.manage(Mutex::new(AudioStreamManager::new(
        handle.state::<Arc<Sender<NetworkPacket>>>().inner().clone(),
        handle.state::<Arc<Receiver<AudioPacket>>>().inner().clone(),
        handle.clone()
    )));

    Ok(())
}

// Maps the current player information to the Audio Output Stream
async fn update_current_player(
    app: AppHandle,
    asm: State<'_, Mutex<AudioStreamManager>>
) -> Result<(), ()>{
    info!("Updating current player metadata");
    match app.store("store.json") {
        Ok(store) => match store.get("current_player") {
            Some(value) => match value.as_str() {
                Some(value) => {
                    _ = update_stream_metadata(String::from("current_player"), String::from(value), AudioDeviceType::OutputDevice, asm.clone()).await;
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

// Toggle mutes a given input stream
#[tauri::command]
pub(crate) async fn mute(
    device: AudioDeviceType,
    asm: State<'_, Mutex<AudioStreamManager>>,
) -> Result<(), ()>{
    let mut asm = asm.lock().await;
    _ = asm.mute(&device).await;

    Ok(())
}

#[tauri::command]
pub(crate) async fn mute_status(
    device: AudioDeviceType,
    asm: State<'_, Mutex<AudioStreamManager>>,
) -> Result<bool, ()> {
    let mut asm = asm.lock().await;
    match asm.mute_status(&device).await {
        Ok(status) => Ok(status),
        Err(_) => Err(()),  
    }
}

#[tauri::command]
pub(crate) async fn is_stopped(
    device: AudioDeviceType,
    asm: State<'_, Mutex<AudioStreamManager>>,
) -> Result<bool, ()> {
    let mut asm = asm.lock().await;
    match asm.is_stopped(&device).await {
        Ok(status) => Ok(status),
        Err(_) => Err(()),  
    }
}