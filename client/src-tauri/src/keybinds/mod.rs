pub(crate) mod listener;

use common::structs::keybinds::{KeybindAction, KeybindConfig, VoiceMode};
use listener::KeybindListener;
use log::{error, info};
use std::sync::Arc;
use tauri::AppHandle;
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut};

pub type ActionMap = Vec<(Shortcut, KeybindAction)>;

pub struct KeybindManager {
    app_handle: AppHandle,
    listener: Arc<KeybindListener>,
    action_map: Arc<parking_lot::RwLock<ActionMap>>,
}

impl KeybindManager {
    pub fn new(
        app_handle: AppHandle,
        listener: Arc<KeybindListener>,
        action_map: Arc<parking_lot::RwLock<ActionMap>>,
    ) -> Self {
        Self {
            app_handle,
            listener,
            action_map,
        }
    }

    pub async fn start(&self, config: KeybindConfig) {
        // Voice mode transition
        self.listener.handle_voice_mode_transition(&config).await;

        // Unregister all existing shortcuts
        let gs = self.app_handle.global_shortcut();
        if let Err(e) = gs.unregister_all() {
            error!("Failed to unregister shortcuts: {:?}", e);
        }

        // Build new shortcuts from config
        let mut entries = Vec::new();

        if config.voice_mode == VoiceMode::OpenMic {
            if let Some(s) = Self::parse_shortcut(&config.toggle_mute) {
                entries.push((s, KeybindAction::ToggleMute));
            }
        }
        if let Some(s) = Self::parse_shortcut(&config.toggle_deafen) {
            entries.push((s, KeybindAction::ToggleDeafen));
        }
        if let Some(s) = Self::parse_shortcut(&config.toggle_recording) {
            entries.push((s, KeybindAction::ToggleRecording));
        }
        if config.voice_mode == VoiceMode::PushToTalk {
            if let Some(s) = Self::parse_shortcut(&config.push_to_talk) {
                entries.push((s, KeybindAction::PushToTalk));
            }
        }

        // Register shortcuts
        for (shortcut, action) in &entries {
            if let Err(e) = gs.register(shortcut.clone()) {
                error!(
                    "Failed to register {:?} for {:?}: {:?}",
                    shortcut, action, e
                );
            }
        }

        info!("Registered {} keybind shortcuts", entries.len());
        *self.action_map.write() = entries;
    }

    fn parse_shortcut(combo: &str) -> Option<Shortcut> {
        let parts: Vec<&str> = combo.split('+').collect();
        let mut mods = Modifiers::empty();
        let mut code: Option<Code> = None;

        for part in parts {
            match part {
                "ControlLeft" | "ControlRight" => mods |= Modifiers::CONTROL,
                "Alt" | "AltGr" => mods |= Modifiers::ALT,
                "ShiftLeft" | "ShiftRight" => mods |= Modifiers::SHIFT,
                "MetaLeft" | "MetaRight" => mods |= Modifiers::META,
                key => {
                    code = key.parse::<Code>().ok();
                    if code.is_none() {
                        error!("Unknown key code: {}", key);
                    }
                }
            }
        }

        let code = code?;
        Some(Shortcut::new(
            if mods.is_empty() { None } else { Some(mods) },
            code,
        ))
    }
}
