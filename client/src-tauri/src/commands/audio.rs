use crate::audio::types::{AudioDevice, AudioDeviceType};
use crate::audio::{AudioPacket, RecordingManager};
use crate::network::NetworkPacket;
use crate::{structs::app_state::AppState, AudioStreamManager};
use common::structs::audio::StreamEvent;
use flume::{Receiver, Sender};
use log::info;
use std::sync::Arc;
use std::{collections::HashMap, time::Duration};
use tauri::async_runtime::Mutex;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_store::StoreExt;

/// Returns the active audio device for the given device type
/// For input devices, this lazily initializes the device if permissions are granted
#[tauri::command]
pub(crate) async fn get_audio_device(
    io: AudioDeviceType,
    state: State<'_, Mutex<AppState>>,
) -> Result<AudioDevice, String> {
    let mut state = state.lock().await;
    state.get_audio_device(io)
}

/// Sets the audio device for a given device type in the application store state
/// For input devices, this verifies permissions before allowing the change
#[tauri::command]
pub(crate) async fn set_audio_device(
    device: AudioDevice,
    app: AppHandle,
    state: State<'_, Mutex<AppState>>,
    asm: State<'_, Mutex<AudioStreamManager>>,
) -> Result<(), String> {
    let mut state = state.lock().await;
    _ = update_current_player(app.clone(), asm.clone());
    state.change_audio_device(device.clone())
}

/// Hard resets the audio stream manager with the new devices
/// For input devices, this lazily initializes the device if permissions are granted
#[tauri::command]
pub(crate) async fn change_audio_device(
    app: AppHandle,
    state: State<'_, Mutex<AppState>>,
    asm: State<'_, Mutex<AudioStreamManager>>,
) -> Result<(), String> {
    let mut state = state.lock().await;
    let mut asm_active = asm.lock().await;

    // Reset the AudioStreamManager
    _ = asm_active.reset().await;

    // Reinitialize the input and output devices
    // Input device will be lazily initialized here if permissions are granted
    let input_device = state.get_audio_device(AudioDeviceType::InputDevice)?;
    let output_device = state.get_audio_device(AudioDeviceType::OutputDevice)?;

    asm_active.init(input_device.clone()).await;
    _ = asm_active.start(input_device.clone().io).await;
    asm_active.init(output_device.clone()).await;
    _ = asm_active.start(output_device.clone().io).await;

    drop(asm_active);

    _ = update_current_player(app.clone(), asm.clone());

    Ok(())
}

#[tauri::command]
pub(crate) async fn update_stream_metadata(
    key: String,
    value: String,
    device: AudioDeviceType,
    asm: State<'_, Mutex<AudioStreamManager>>,
) -> Result<(), ()> {
    let mut asm = asm.lock().await;
    _ = asm.metadata(key, value, &device).await;

    Ok(())
}

#[tauri::command]
pub(crate) async fn reset_asm(
    asm: State<'_, Mutex<AudioStreamManager>>,
) -> Result<(), ()> {
    let mut asm = asm.lock().await;
    _ = asm.reset().await;
    Ok(())
}

// Maps the current player information to the Audio Output Stream
async fn update_current_player(
    app: AppHandle,
    asm: State<'_, Mutex<AudioStreamManager>>,
) -> Result<(), ()> {
    info!("Updating current player metadata");
    match app.store("store.json") {
        Ok(store) => match store.get("current_player") {
            Some(value) => match value.as_str() {
                Some(value) => {
                    _ = update_stream_metadata(
                        String::from("current_player"),
                        String::from(value),
                        AudioDeviceType::OutputDevice,
                        asm.clone(),
                    )
                    .await;
                }
                None => return Err(()),
            },
            None => return Err(()),
        },
        Err(_) => return Err(()),
    };

    Ok(())
}

/// Stops the audio stream for a given device
/// This will trigger a "stop-audio-device" event
#[tauri::command]
pub(crate) async fn stop_audio_device(
    device: AudioDeviceType,
    asm: State<'_, Mutex<AudioStreamManager>>,
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
    recording_manager: State<'_, Arc<Mutex<RecordingManager>>>,
    broadcaster: State<'_, crate::websocket::WebSocketBroadcaster>,
) -> Result<(), ()> {
    let mut asm = asm.lock().await;
    _ = asm.toggle(&device, StreamEvent::Mute).await;

    // Broadcast state to all WS clients
    let muted = asm.mute_status(&AudioDeviceType::InputDevice).await.unwrap_or(false);
    let deafened = asm.mute_status(&AudioDeviceType::OutputDevice).await.unwrap_or(false);
    drop(asm);

    let manager = recording_manager.lock().await;
    let recording = manager.is_recording();
    drop(manager);

    let response = crate::websocket::SuccessResponse::state(muted, deafened, recording);
    if let Ok(json) = serde_json::to_string(&response) {
        let _ = broadcaster.0.send(json);
    }

    Ok(())
}

#[tauri::command]
pub(crate) async fn record(
    asm: State<'_, Mutex<AudioStreamManager>>,
) -> Result<(), ()> {
    let mut asm = asm.lock().await;
    _ = asm.toggle(&AudioDeviceType::InputDevice, StreamEvent::Record).await;
    _ = asm.toggle(&AudioDeviceType::OutputDevice, StreamEvent::Record).await;

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

/// Start recording session
#[tauri::command]
pub(crate) async fn start_recording(
    app: AppHandle,
    recording_manager: State<'_, Arc<Mutex<RecordingManager>>>,
    asm: State<'_, Mutex<AudioStreamManager>>,
    broadcaster: State<'_, crate::websocket::WebSocketBroadcaster>,
) -> Result<String, String> {
    let current_player = extract_current_player(&app).await
        .ok_or_else(|| "No current player set for recording".to_string())?;

    // Recording is now controlled by RecordingManager's shared flag
    // No need to toggle stream recording separately
    let mut manager = recording_manager.lock().await;
    let result = match manager.start_recording(current_player).await {
        Ok(_) => {
            if let Some(session_id) = manager.current_session_id() {
                Ok(session_id)
            } else {
                Err("Recording started but no session ID available".to_string())
            }
        },
        Err(e) => Err(format!("Failed to start recording: {:?}", e)),
    };

    // Broadcast state to all WS clients
    let recording = manager.is_recording();
    drop(manager);

    let mut asm = asm.lock().await;
    let muted = asm.mute_status(&AudioDeviceType::InputDevice).await.unwrap_or(false);
    let deafened = asm.mute_status(&AudioDeviceType::OutputDevice).await.unwrap_or(false);
    drop(asm);

    let response = crate::websocket::SuccessResponse::state(muted, deafened, recording);
    if let Ok(json) = serde_json::to_string(&response) {
        let _ = broadcaster.0.send(json);
    }

    result
}

/// Stop current recording session
#[tauri::command]
pub(crate) async fn stop_recording(
    recording_manager: State<'_, Arc<Mutex<RecordingManager>>>,
    asm: State<'_, Mutex<AudioStreamManager>>,
    broadcaster: State<'_, crate::websocket::WebSocketBroadcaster>,
) -> Result<(), String> {
    // Recording is now controlled by RecordingManager's shared flag
    // No need to toggle stream recording separately
    let mut manager = recording_manager.lock().await;
    let result = match manager.stop_recording().await {
        Ok(()) => Ok(()),
        Err(e) => Err(format!("Failed to stop recording: {:?}", e)),
    };

    // Broadcast state to all WS clients
    let recording = manager.is_recording();
    drop(manager);

    let mut asm = asm.lock().await;
    let muted = asm.mute_status(&AudioDeviceType::InputDevice).await.unwrap_or(false);
    let deafened = asm.mute_status(&AudioDeviceType::OutputDevice).await.unwrap_or(false);
    drop(asm);

    let response = crate::websocket::SuccessResponse::state(muted, deafened, recording);
    if let Ok(json) = serde_json::to_string(&response) {
        let _ = broadcaster.0.send(json);
    }

    result
}

/// Get current recording status
#[tauri::command]
pub(crate) async fn get_recording_status(
    recording_manager: State<'_, Arc<Mutex<RecordingManager>>>,
) -> Result<serde_json::Value, String> {
    let manager = recording_manager.lock().await;
    let is_recording = manager.is_recording();
    let session_id = manager.current_session_id();

    Ok(serde_json::json!({
        "is_recording": is_recording,
        "session_id": session_id
    }))
}

/// Check if recording is currently active (simple boolean query)
#[tauri::command]
pub(crate) async fn is_recording(
    recording_manager: State<'_, Arc<Mutex<RecordingManager>>>,
) -> Result<bool, String> {
    let manager = recording_manager.lock().await;
    Ok(manager.is_recording())
}

/// Helper function to extract current player from app metadata
async fn extract_current_player(app: &AppHandle) -> Option<String> {
    app.store("store.json").ok()?
        .get("current_player")?
        .as_str()
        .map(String::from)
}

/// Returns the list of currently tracked players from the output stream's presence cache
#[tauri::command]
pub(crate) async fn get_current_players(
    asm: State<'_, Mutex<AudioStreamManager>>,
) -> Result<Vec<String>, ()> {
    let asm = asm.lock().await;
    Ok(asm.get_current_players())
}

/// Restart audio stream after error recovery
/// This can be called by the frontend after receiving an audio-stream-recovery event
#[tauri::command]
pub(crate) async fn restart_audio_stream(
    device: AudioDeviceType,
    asm: State<'_, Mutex<AudioStreamManager>>,
) -> Result<(), String> {
    info!("Restarting audio stream for {:?}", device);
    let mut asm = asm.lock().await;

    asm.restart(device).await.map_err(|e| {
        let err_msg = format!("Failed to restart audio stream: {:?}", e);
        log::error!("{}", err_msg);
        err_msg
    })
}
