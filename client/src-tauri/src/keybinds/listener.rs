use crate::audio::types::AudioDeviceType;
use crate::audio::AudioActionsManager;
use common::structs::keybinds::{KeybindAction, KeybindConfig, PttEvent, VoiceMode};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

pub struct KeybindListener {
    app_handle: AppHandle,
    ptt_held: AtomicBool,
}

impl KeybindListener {
    pub fn new(app_handle: AppHandle) -> Self {
        Self {
            app_handle,
            ptt_held: AtomicBool::new(false),
        }
    }

    pub async fn handle_voice_mode_transition(&self, config: &KeybindConfig) {
        self.ptt_held.store(false, Ordering::Relaxed);
        let actions = self.app_handle.state::<AudioActionsManager>();
        let is_muted = actions.is_muted(AudioDeviceType::InputDevice).await;
        match config.voice_mode {
            VoiceMode::PushToTalk if !is_muted => {
                actions.toggle_mute(AudioDeviceType::InputDevice).await;
            }
            VoiceMode::OpenMic if is_muted => {
                actions.toggle_mute(AudioDeviceType::InputDevice).await;
            }
            _ => {}
        }
    }

    pub async fn on_action_press(&self, action: KeybindAction) {
        match action {
            KeybindAction::ToggleMute => self.dispatch_toggle_mute().await,
            KeybindAction::ToggleDeafen => self.dispatch_toggle_deafen().await,
            KeybindAction::ToggleRecording => self.dispatch_toggle_recording().await,
            KeybindAction::PushToTalk => self.dispatch_ptt_press().await,
        }
    }

    pub async fn on_action_release(&self, action: KeybindAction) {
        if action == KeybindAction::PushToTalk {
            self.dispatch_ptt_release().await;
        }
    }

    async fn dispatch_toggle_mute(&self) {
        let actions = self.app_handle.state::<AudioActionsManager>();
        actions.toggle_mute(AudioDeviceType::InputDevice).await;
        actions.broadcast_state().await;
    }

    async fn dispatch_toggle_deafen(&self) {
        let actions = self.app_handle.state::<AudioActionsManager>();
        actions.toggle_mute(AudioDeviceType::OutputDevice).await;
        actions.broadcast_state().await;
    }

    async fn dispatch_toggle_recording(&self) {
        let actions = self.app_handle.state::<AudioActionsManager>();
        let _ = actions.toggle_recording().await;
        actions.broadcast_state().await;
    }

    async fn dispatch_ptt_press(&self) {
        if self.ptt_held.swap(true, Ordering::Relaxed) {
            return;
        }
        let actions = self.app_handle.state::<AudioActionsManager>();
        if actions.is_muted(AudioDeviceType::InputDevice).await {
            actions.toggle_mute(AudioDeviceType::InputDevice).await;
        }
        self.app_handle.emit(&PttEvent::Active.to_string(), true).ok();
        actions.broadcast_state().await;
    }

    async fn dispatch_ptt_release(&self) {
        self.ptt_held.store(false, Ordering::Relaxed);
        tokio::time::sleep(Duration::from_millis(300)).await;
        if !self.ptt_held.load(Ordering::Relaxed) {
            let actions = self.app_handle.state::<AudioActionsManager>();
            if !actions.is_muted(AudioDeviceType::InputDevice).await {
                actions.toggle_mute(AudioDeviceType::InputDevice).await;
            }
            self.app_handle.emit(&PttEvent::Active.to_string(), false).ok();
            actions.broadcast_state().await;
        }
    }
}
