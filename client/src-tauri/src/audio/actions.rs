use crate::audio::types::AudioDeviceType;
use crate::audio::{AudioStreamManager, RecordingManager};
use common::structs::audio::{MuteEvent, StreamEvent};
use log::info;
use std::sync::Arc;
use tauri::async_runtime::Mutex;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_store::StoreExt;

pub(crate) struct AudioActionsManager {
    app_handle: AppHandle,
}

impl AudioActionsManager {
    pub fn new(app_handle: AppHandle) -> Self {
        Self { app_handle }
    }

    /// Toggle mute for a device, emit `mute:{device}` event, return new mute status.
    pub async fn toggle_mute(&self, device: AudioDeviceType) -> bool {
        let asm = self
            .app_handle
            .state::<Mutex<AudioStreamManager>>();
        let mut asm = asm.lock().await;
        let _ = asm.toggle(&device, StreamEvent::Mute).await;
        let status = asm.mute_status(&device).await.unwrap_or(false);
        drop(asm);

        let mute_event = MuteEvent::from(&device);
        self.app_handle
            .emit(&mute_event.to_string(), status)
            .ok();

        info!("{} {}", mute_event, if status { "muted" } else { "unmuted" });

        status
    }

    /// Query mute status without toggling.
    pub async fn is_muted(&self, device: AudioDeviceType) -> bool {
        let asm = self
            .app_handle
            .state::<Mutex<AudioStreamManager>>();
        let mut asm = asm.lock().await;
        asm.mute_status(&device).await.unwrap_or(false)
    }

    /// Toggle recording on/off. Returns new recording state.
    pub async fn toggle_recording(&self) -> Result<bool, anyhow::Error> {
        let recording_manager = self
            .app_handle
            .state::<Arc<Mutex<RecordingManager>>>();
        let mut manager = recording_manager.lock().await;

        if manager.is_recording() {
            manager.stop_recording().await?;
            Ok(false)
        } else {
            let current_player = self
                .app_handle
                .store("store.json")
                .ok()
                .and_then(|store| store.get("current_player"))
                .and_then(|v| v.as_str().map(String::from))
                .ok_or_else(|| anyhow::anyhow!("No current player"))?;

            manager.start_recording(current_player).await?;
            Ok(true)
        }
    }

    /// Query current muted/deafened/recording state as a DTO.
    pub async fn query_state(&self) -> crate::websocket::StateData {
        let asm = self
            .app_handle
            .state::<Mutex<AudioStreamManager>>();
        let mut asm = asm.lock().await;
        let muted = asm
            .mute_status(&AudioDeviceType::InputDevice)
            .await
            .unwrap_or(false);
        let deafened = asm
            .mute_status(&AudioDeviceType::OutputDevice)
            .await
            .unwrap_or(false);
        drop(asm);

        let recording_manager = self
            .app_handle
            .state::<Arc<Mutex<RecordingManager>>>();
        let manager = recording_manager.lock().await;
        let recording = manager.is_recording();
        drop(manager);

        crate::websocket::StateData {
            muted,
            deafened,
            recording,
        }
    }

    /// Query state and broadcast to all WS clients.
    pub async fn broadcast_state(&self) {
        let state = self.query_state().await;
        let broadcaster = self
            .app_handle
            .state::<crate::websocket::WebSocketBroadcaster>();
        broadcaster.broadcast_state(state);
    }
}
