use common::structs::audio::VoiceRuntimeState;
use common::structs::keybinds::KeybindConfig;
use std::sync::Arc;

use crate::keybinds::KeybindListener;

/// Apply a keybind config, and report what the backend reached.
///
/// On desktop this also registers the global shortcuts. On every platform it runs the
/// voice-mode transition, which is what mutes the input on the way into push-to-talk —
/// without it the mode is a label on a screen and the microphone stays open.
///
/// The reached state comes back rather than being announced, because the announcement did
/// not arrive on Android: the settings control moved, the backend did or did not follow,
/// and nothing compared the two. A caller that reads this cannot show a mode the backend
/// is not in.
#[cfg(desktop)]
#[tauri::command]
pub(crate) async fn start_keybind_listener(
    config: KeybindConfig,
    km: tauri::State<'_, crate::keybinds::KeybindManager>,
    actions: tauri::State<'_, crate::audio::AudioActionsManager>,
) -> Result<VoiceRuntimeState, String> {
    km.start(config).await;
    Ok(actions.voice_runtime_state().await)
}

#[cfg(not(desktop))]
#[tauri::command]
pub(crate) async fn start_keybind_listener(
    config: KeybindConfig,
    listener: tauri::State<'_, Arc<KeybindListener>>,
    actions: tauri::State<'_, crate::audio::AudioActionsManager>,
) -> Result<VoiceRuntimeState, String> {
    listener.handle_voice_mode_transition(&config).await;
    Ok(actions.voice_runtime_state().await)
}

/// Push-to-talk held, from a surface that is not a global hotkey.
///
/// A phone has no hotkey, so the mic button is the only way in and this is the only route
/// it has. Ignored outside push-to-talk, so a caller cannot use it to open the microphone
/// in open mic.
#[tauri::command]
pub(crate) async fn set_ptt(
    down: bool,
    listener: tauri::State<'_, Arc<KeybindListener>>,
) -> Result<(), String> {
    listener.set_ptt(down).await;
    Ok(())
}

/// What the backend believes about the microphone, for the diagnostics readout.
///
/// A muted input and a capture stream that has stopped emitting draw the same flat meter,
/// and in push-to-talk the mic button never shows the muted glyph. Neither the meter nor
/// the button can answer "is the microphone open"; this can.
#[tauri::command]
pub(crate) async fn voice_runtime_state(
    actions: tauri::State<'_, crate::audio::AudioActionsManager>,
) -> Result<VoiceRuntimeState, String> {
    Ok(actions.voice_runtime_state().await)
}
