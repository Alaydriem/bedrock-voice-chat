use crate::audio::AudioActionsManager;
use crate::audio::AudioDeviceType;
use common::structs::keybinds::{
    KeybindAction, KeybindConfig, PttEvent, VoiceMode, VoiceModeEvent,
};
use log::info;
use super::PttHold;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, async_runtime::Mutex};

/// How long the microphone stays open after the hold is released, so key repeat and the
/// gap between two words do not clip the end of a sentence.
const PTT_TAIL: Duration = Duration::from_millis(300);

pub struct KeybindListener {
    app_handle: AppHandle,
    hold: PttHold,
    last_voice_mode: Mutex<Option<VoiceMode>>,
}

impl KeybindListener {
    pub fn new(app_handle: AppHandle) -> Self {
        Self {
            app_handle,
            hold: PttHold::new(),
            last_voice_mode: Mutex::new(None),
        }
    }

    /// The mode in force, for anything that has to know what the mute control means.
    pub async fn voice_mode(&self) -> VoiceMode {
        self.last_voice_mode
            .lock()
            .await
            .clone()
            .unwrap_or(VoiceMode::OpenMic)
    }

    pub fn is_ptt_held(&self) -> bool {
        self.hold.is_held()
    }

    /// Push-to-talk from a surface that is not a global hotkey: the mic button, or a
    /// Stream Deck key held down.
    pub async fn set_ptt(&self, down: bool) {
        if down {
            self.dispatch_ptt_press().await;
        } else {
            self.dispatch_ptt_release().await;
        }
    }

    pub async fn handle_voice_mode_transition(&self, config: &KeybindConfig) {
        self.hold.clear();

        // Only run the transition if the voice mode actually changed
        let mut last = self.last_voice_mode.lock().await;
        if last.as_ref() == Some(&config.voice_mode) {
            info!(
                "Voice mode unchanged ({:?}), skipping transition",
                config.voice_mode
            );
            return;
        }
        let previous = last.clone();
        *last = Some(config.voice_mode.clone());
        drop(last);

        info!(
            "Voice mode transition: {:?} -> {:?}",
            previous, config.voice_mode
        );

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

        // Whoever changed the mode, every surface showing a mute control has to change
        // what that control does. Read once at start-up, a window keeps offering a toggle
        // for a mode where holding is the only thing that transmits.
        self.app_handle
            .emit(&VoiceModeEvent::Changed.to_string(), &config.voice_mode)
            .ok();
        actions.broadcast_state().await;
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
        // Holding is meaningless in open mic, and unmuting on it would be a hotkey that
        // silently turns the microphone on.
        if self.voice_mode().await != VoiceMode::PushToTalk {
            return;
        }
        if !self.hold.press() {
            return;
        }
        let actions = self.app_handle.state::<AudioActionsManager>();
        if actions.is_muted(AudioDeviceType::InputDevice).await {
            actions.toggle_mute(AudioDeviceType::InputDevice).await;
        }
        self.app_handle
            .emit(&PttEvent::Active.to_string(), true)
            .ok();
        actions.broadcast_state().await;
    }

    /// Release the hold, and close the microphone a beat later.
    ///
    /// The tail rides over key repeat and over the gap between syllables, so the last word
    /// is not clipped. It runs detached because the caller may be a Stream Deck's command
    /// loop: awaited, a release would hold that connection for the whole tail and the next
    /// press of a double-tap would arrive after it.
    async fn dispatch_ptt_release(&self) {
        // A release only answers a press this object registered. Unpaired — a tap whose
        // press was refused because the mode was not push-to-talk, or a controller sending
        // one on connect — it used to run anyway and mute the input a beat later, which on
        // a phone read as a microphone that died when the button was tapped.
        if !self.hold.release() {
            return;
        }

        let app_handle = self.app_handle.clone();
        let hold = self.hold.clone();
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(PTT_TAIL).await;
            if !hold.tail_should_close() {
                return;
            }
            let actions = app_handle.state::<AudioActionsManager>();
            if !actions.is_muted(AudioDeviceType::InputDevice).await {
                actions.toggle_mute(AudioDeviceType::InputDevice).await;
            }
            app_handle.emit(&PttEvent::Active.to_string(), false).ok();
            actions.broadcast_state().await;
        });
    }
}
