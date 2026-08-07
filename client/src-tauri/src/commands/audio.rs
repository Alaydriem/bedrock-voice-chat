use crate::audio::types::{AudioDevice, AudioDeviceType};
use crate::audio::{AudioActionsManager, RecordingManager};
use crate::events::event::notification::{EVENT_NOTIFICATION, Notification};
use crate::{AudioStreamManager, structs::app_state::AppState};
use common::structs::audio::StreamEvent;
use log::{error, info, warn};
use std::collections::HashMap;
use std::sync::Arc;
use tauri::Emitter;
use tauri::async_runtime::Mutex;
use tauri::{AppHandle, State};
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
    let _ = update_current_player(app.clone(), asm.clone()).await;
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
    // Phase 1: Get devices (short state lock, released before slow operations)
    let (input_device, output_device) = {
        let mut state = state.lock().await;
        let input = state.get_audio_device(AudioDeviceType::InputDevice)?;
        let output = state.get_audio_device(AudioDeviceType::OutputDevice)?;
        (input, output)
    };

    // Phase 2: init/start streams (asm lock only). `init` stops and rebuilds each stream with its
    // device, so a reset first would build two device-less streams that are never started.
    let mut asm_active = asm.lock().await;

    // Input device: init, start, fallback to default on failure
    asm_active.init(input_device.clone()).await;
    if let Err(e) = asm_active.start(input_device.clone().io).await {
        warn!(
            "Input device '{}' failed: {}. Falling back to default.",
            input_device.display_name, e
        );
        drop(asm_active);
        let fallback_result = {
            let mut state = state.lock().await;
            state
                .clear_audio_device(AudioDeviceType::InputDevice)
                .and_then(|_| state.get_audio_device(AudioDeviceType::InputDevice))
        };
        match fallback_result {
            Ok(default_input) => {
                asm_active = asm.lock().await;
                asm_active.init(default_input.clone()).await;
                if let Err(e2) = asm_active.start(default_input.clone().io).await {
                    error!("Default input device also failed: {}", e2);
                    return Err(format!("NO_INPUT_DEVICE: {}", e2));
                }
                let _ = app.emit(
                    EVENT_NOTIFICATION,
                    Notification::new(
                        "Input Device Unavailable".to_string(),
                        format!(
                            "'{}' could not be activated. Switched to default device.",
                            input_device.display_name
                        ),
                        Some("warning".to_string()),
                        None,
                        None,
                        None,
                    ),
                );
            }
            Err(e) if e.contains("INCOMPATIBLE_DEVICE") => {
                error!("Incompatible input device: {}", e);
                return Err(e);
            }
            Err(e) => {
                error!("No input device available: {}", e);
                return Err(format!("NO_INPUT_DEVICE: {}", e));
            }
        }
    }

    // Output device: init, start, fallback to default on failure
    asm_active.init(output_device.clone()).await;
    if let Err(e) = asm_active.start(output_device.clone().io).await {
        warn!(
            "Output device '{}' failed: {}. Falling back to default.",
            output_device.display_name, e
        );
        drop(asm_active);
        let fallback_result = {
            let mut state = state.lock().await;
            state
                .clear_audio_device(AudioDeviceType::OutputDevice)
                .and_then(|_| state.get_audio_device(AudioDeviceType::OutputDevice))
        };
        match fallback_result {
            Ok(default_output) => {
                asm_active = asm.lock().await;
                asm_active.init(default_output.clone()).await;
                if let Err(e2) = asm_active.start(default_output.clone().io).await {
                    error!("Default output device also failed: {}", e2);
                    return Err(format!("NO_OUTPUT_DEVICE: {}", e2));
                }
                let _ = app.emit(
                    EVENT_NOTIFICATION,
                    Notification::new(
                        "Output Device Unavailable".to_string(),
                        format!(
                            "'{}' could not be activated. Switched to default device.",
                            output_device.display_name
                        ),
                        Some("warning".to_string()),
                        None,
                        None,
                        None,
                    ),
                );
            }
            Err(e) if e.contains("INCOMPATIBLE_DEVICE") => {
                error!("Incompatible output device: {}", e);
                return Err(e);
            }
            Err(e) => {
                error!("No output device available: {}", e);
                return Err(format!("NO_OUTPUT_DEVICE: {}", e));
            }
        }
    }

    drop(asm_active);
    let _ = update_current_player(app.clone(), asm.clone()).await;

    // Rebuilding the output stream constructs a fresh, empty `GainProjection`, and the
    // preserved metadata cache does not carry it — the `player_gain_store` arm hands the store
    // straight to the projection and never caches it. Without this re-seed every player the
    // user muted becomes audible again for the rest of the session while their card still says
    // muted. Only after the lock is released: `publish` takes it itself.
    crate::players::PlayerSettingsCoordinator::reseed(&app).await;

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
    app: AppHandle,
    asm: State<'_, Mutex<AudioStreamManager>>,
) -> Result<(), ()> {
    {
        let mut asm = asm.lock().await;
        _ = asm.restart_session().await;
    }

    // A reset builds a new output stream with an empty gain projection. Every caller today
    // happens to be followed by something that re-seeds — a cold boot, or a sign-out that ends
    // the session — so this is currently belt and braces. It is here anyway because relying on
    // that sequencing is exactly the shape of the bug that made every mute stop applying after
    // an output-device change.
    crate::players::PlayerSettingsCoordinator::reseed(&app).await;
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
    actions: State<'_, AudioActionsManager>,
) -> Result<(), ()> {
    actions.toggle_mute(device).await;
    actions.broadcast_state().await;
    Ok(())
}

/// Drive a device to an absolute state.
///
/// A toggle is enough for a button press, but not for reconciling with a state something
/// else set: deciding whether to flip requires reading first, and a hotkey firing between
/// the read and the flip leaves the two permanently inverted. Returns the state actually
/// reached, though every surface learns it from the `mute:*` event either way.
#[tauri::command]
pub(crate) async fn set_mute(
    device: AudioDeviceType,
    muted: bool,
    actions: State<'_, AudioActionsManager>,
) -> Result<bool, String> {
    // At info so it lands in logcat on a device build: "did my press reach Rust at all" is the
    // first question when a button appears to do nothing, and it cannot be answered from the
    // webview alone.
    info!("set_mute({:?}, {}) requested", device, muted);
    let status = actions.set_mute(device.clone(), muted).await;
    info!("set_mute({:?}) resolved to {}", device, status);
    actions.broadcast_state().await;
    Ok(status)
}

/// Deafen, which also drives the input — see `AudioActionsManager::set_deafened`.
#[tauri::command]
pub(crate) async fn set_deafened(
    deafened: bool,
    actions: State<'_, AudioActionsManager>,
) -> Result<bool, String> {
    info!("set_deafened({}) requested", deafened);
    let status = actions.set_deafened(deafened).await;
    info!("set_deafened resolved to {}", status);
    actions.broadcast_state().await;
    Ok(status)
}

#[tauri::command]
pub(crate) async fn record(asm: State<'_, Mutex<AudioStreamManager>>) -> Result<(), ()> {
    let mut asm = asm.lock().await;
    _ = asm
        .toggle(&AudioDeviceType::InputDevice, StreamEvent::Record)
        .await;
    _ = asm
        .toggle(&AudioDeviceType::OutputDevice, StreamEvent::Record)
        .await;

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
    actions: State<'_, AudioActionsManager>,
) -> Result<String, String> {
    let current_player = extract_current_player(&app)
        .await
        .ok_or_else(|| "No current player set for recording".to_string())?;

    let mut manager = recording_manager.lock().await;
    let result = match manager.start_recording(current_player).await {
        Ok(_) => {
            if let Some(session_id) = manager.current_session_id() {
                Ok(session_id)
            } else {
                Err("Recording started but no session ID available".to_string())
            }
        }
        Err(e) => Err(format!("Failed to start recording: {:?}", e)),
    };
    drop(manager);

    actions.broadcast_state().await;

    result
}

/// Stop current recording session
#[tauri::command]
pub(crate) async fn stop_recording(
    recording_manager: State<'_, Arc<Mutex<RecordingManager>>>,
    actions: State<'_, AudioActionsManager>,
) -> Result<(), String> {
    let mut manager = recording_manager.lock().await;
    let result = match manager.stop_recording().await {
        Ok(()) => Ok(()),
        Err(e) => Err(format!("Failed to stop recording: {:?}", e)),
    };
    drop(manager);

    actions.broadcast_state().await;

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
    app.store("store.json")
        .ok()?
        .get("current_player")?
        .as_str()
        .map(String::from)
}

/// Returns the currently tracked players with their game type
#[tauri::command]
pub(crate) async fn get_current_players(
    asm: State<'_, Mutex<AudioStreamManager>>,
) -> Result<std::collections::HashMap<String, Option<String>>, ()> {
    let asm = asm.lock().await;
    Ok(asm.get_current_players())
}

/// Restart audio stream after error recovery
/// This can be called by the frontend after receiving an audio-stream-recovery event
#[tauri::command]
pub(crate) async fn restart_audio_stream(
    app: AppHandle,
    device: AudioDeviceType,
    asm: State<'_, Mutex<AudioStreamManager>>,
) -> Result<(), String> {
    info!("Restarting audio stream for {:?}", device);
    let restarted = {
        let mut asm = asm.lock().await;
        asm.restart(device.clone()).await.map_err(|e| {
            let err_msg = format!("Failed to restart audio stream: {:?}", e);
            log::error!("{}", err_msg);
            err_msg
        })
    };
    restarted?;

    // A restarted output stream carries a fresh, empty gain projection. Re-seeded outside the
    // lock, because `publish` acquires it.
    if matches!(device, AudioDeviceType::OutputDevice) {
        crate::players::PlayerSettingsCoordinator::reseed(&app).await;
    }
    Ok(())
}

/// Capture only to drive the level meter on the setup screen. Emits
/// `audio-input-level` exactly as a session stream does, and transmits nothing.
///
/// The device comes from `AppState`, which is the same place the device selector reads
/// its preselected value from. The stream manager keeps its own copy and has none until
/// `init`, so metering the selection means handing it over rather than assuming the
/// manager already knows it.
/// Whether a session capture stream is already running.
///
/// The settings meter asks this before starting one of its own. Inferring it from the arrival
/// of level events cost a live capture every time somebody opened the audio pane once levels
/// stopped being published on a fixed clock.
#[tauri::command]
pub(crate) async fn input_capture_active(
    asm: State<'_, Mutex<AudioStreamManager>>,
) -> Result<bool, String> {
    Ok(asm.lock().await.input_capture_active())
}

/// One `audio-levels` message, now, regardless of what the emit policy would send.
///
/// The verification half of the webview's listener handshake: a listener that just registered
/// invokes this and waits for the snapshot to arrive through itself. Not receiving it is the
/// only way the page can tell a phantom registration from a quiet room, because the emit
/// policy never re-sends silence.
#[tauri::command]
pub(crate) async fn probe_audio_levels(
    asm: State<'_, Mutex<AudioStreamManager>>,
) -> Result<(), String> {
    asm.lock()
        .await
        .publish_levels_now()
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub(crate) async fn start_input_meter(
    state: State<'_, Mutex<AppState>>,
    asm: State<'_, Mutex<AudioStreamManager>>,
) -> Result<(), String> {
    let device = {
        let mut state = state.lock().await;
        state.get_audio_device(AudioDeviceType::InputDevice)?
    };

    let mut asm = asm.lock().await;
    asm.start_input_metering(device)
        .await
        .map_err(|e| format!("Failed to start input meter: {:?}", e))
}

/// Play a chime through the selected output device, so the user can confirm they will hear
/// other people. Resolves once the chime has finished, which is what lets the button
/// disable itself for exactly as long as it is playing.
///
/// `spawn_blocking` because the rodio stream is not `Send` and dropping it cuts playback,
/// so it has to live and die on one thread rather than be held across an await.
#[tauri::command]
pub(crate) async fn test_output_device(
    state: State<'_, Mutex<AppState>>,
) -> Result<(), String> {
    let device = {
        let mut state = state.lock().await;
        state.get_audio_device(AudioDeviceType::OutputDevice)?
    };

    tokio::task::spawn_blocking(move || crate::audio::SpeakerTest::new().play(device))
        .await
        .map_err(|e| format!("Speaker test task failed: {:?}", e))?
        .map_err(|e| format!("Could not play through that device: {:?}", e))
}

#[tauri::command]
pub(crate) async fn stop_input_meter(
    asm: State<'_, Mutex<AudioStreamManager>>,
) -> Result<(), String> {
    let mut asm = asm.lock().await;
    asm.stop_input_metering()
        .await
        .map_err(|e| format!("Failed to stop input meter: {:?}", e))
}
