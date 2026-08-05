use crate::audio::types::AudioDeviceType;
use crate::audio::{AudioStreamManager, RecordingManager};
use common::structs::audio::{MuteEvent, StreamEvent};
use log::info;
use std::sync::Arc;
use tauri::async_runtime::Mutex;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_store::StoreExt;

pub struct AudioActionsManager {
    app_handle: AppHandle,
}

impl AudioActionsManager {
    pub fn new(app_handle: AppHandle) -> Self {
        Self { app_handle }
    }

    /// Toggle mute for a device, emit `mute:{device}` event, return new mute status.
    pub async fn toggle_mute(&self, device: AudioDeviceType) -> bool {
        let asm = self.app_handle.state::<Mutex<AudioStreamManager>>();
        let mut asm = asm.lock().await;
        let _ = asm.toggle(&device, StreamEvent::Mute).await;
        let status = asm.mute_status(&device).await.unwrap_or(false);
        drop(asm);

        let mute_event = MuteEvent::from(&device);
        self.app_handle.emit(&mute_event.to_string(), status).ok();

        info!(
            "{} {}",
            mute_event,
            if status { "muted" } else { "unmuted" }
        );

        status
    }

    /// Query mute status without toggling.
    pub async fn is_muted(&self, device: AudioDeviceType) -> bool {
        let asm = self.app_handle.state::<Mutex<AudioStreamManager>>();
        let mut asm = asm.lock().await;
        asm.mute_status(&device).await.unwrap_or(false)
    }

    /// Toggle recording on/off. Returns new recording state.
    pub async fn toggle_recording(&self) -> Result<bool, anyhow::Error> {
        let recording_manager = self.app_handle.state::<Arc<Mutex<RecordingManager>>>();
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

    /// Drive a device to `desired`, flipping only if it differs — the check and the
    /// toggle happen under a single `AudioStreamManager` lock so an idempotent
    /// `set_mute(dev, true)` can't race the desktop-app / Stream-Deck toggle surfaces.
    pub async fn set_mute(&self, device: AudioDeviceType, desired: bool) -> bool {
        let asm = self.app_handle.state::<Mutex<AudioStreamManager>>();
        let mut asm = asm.lock().await;
        if asm.mute_status(&device).await.unwrap_or(false) != desired {
            let _ = asm.toggle(&device, StreamEvent::Mute).await;
        }
        let status = asm.mute_status(&device).await.unwrap_or(false);
        drop(asm);

        let mute_event = MuteEvent::from(&device);
        self.app_handle.emit(&mute_event.to_string(), status).ok();
        status
    }

    /// Deafen, and the mute that has to come with it.
    ///
    /// Hearing nobody while they can still hear you is a state people reach by accident
    /// and cannot detect from their own screen, so deafening drives the input too.
    /// Undeafening clears both, because the fix for "I cannot hear anyone" must not be a
    /// second button they have no reason to suspect.
    ///
    /// The pairing lives here rather than in each caller so the in-game action, the global
    /// hotkey and the app cannot disagree about what deafen means. Each leg still emits its
    /// own `mute:*` event, which is how every surface learns the resulting pair.
    pub async fn set_deafened(&self, desired: bool) -> bool {
        self.set_mute(AudioDeviceType::OutputDevice, desired).await;
        self.set_mute(AudioDeviceType::InputDevice, desired).await;
        desired
    }

    /// Drive recording to `desired`, starting/stopping only if it differs — the
    /// check and the transition happen under a single `RecordingManager` lock.
    pub async fn set_recording(&self, desired: bool) -> Result<bool, anyhow::Error> {
        let recording_manager = self.app_handle.state::<Arc<Mutex<RecordingManager>>>();
        let mut manager = recording_manager.lock().await;

        if manager.is_recording() == desired {
            return Ok(desired);
        }
        if desired {
            let current_player = self
                .app_handle
                .store("store.json")
                .ok()
                .and_then(|store| store.get("current_player"))
                .and_then(|v| v.as_str().map(String::from))
                .ok_or_else(|| anyhow::anyhow!("No current player"))?;
            manager.start_recording(current_player).await?;
        } else {
            manager.stop_recording().await?;
        }
        Ok(desired)
    }

    /// Query current muted/deafened/recording state as a DTO.
    pub async fn query_state(&self) -> crate::websocket::StateData {
        let asm = self.app_handle.state::<Mutex<AudioStreamManager>>();
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

        let recording_manager = self.app_handle.state::<Arc<Mutex<RecordingManager>>>();
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

        // Every mute/deafen/record surface funnels through here; nudge the
        // control-plane reporter so the server cache mirrors the change.
        if let Some(bus) = self.app_handle.try_state::<crate::control::ControlStateBus>() {
            bus.self_state();
        }
    }
}
