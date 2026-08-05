use common::structs::audio::{PlayerGainSettings, PlayerGainStore};
use common::structs::control::ClientActionType;
use log::warn;
use tauri::async_runtime::Mutex;
use tauri::{Emitter, Manager};
use tauri_plugin_store::StoreExt;

use crate::audio::AudioActionsManager;
use crate::audio::AudioStreamManager;
use crate::audio::types::AudioDeviceType;

/// Executes delivered `ClientAction`s against the real desktop managers. Self-state
/// actions route through `AudioActionsManager` (mute/deafen/record); per-player
/// preferences route through the same persisted `player_gain_store` path the
/// dashboard UI uses. Group actions are applied server-side (the client learns via
/// the existing `ChannelEvent`), so they are ignored here.
pub struct ControlActionsManager {
    app_handle: tauri::AppHandle,
}

impl ControlActionsManager {
    pub fn new(app_handle: tauri::AppHandle) -> Self {
        Self { app_handle }
    }

    /// Consume the control-action channel until every sender is dropped. This is
    /// the only place delivered actions meet the `AppHandle`: producers hold a
    /// `ControlActionSender` (a plain DTO channel), keeping the Tauri GUI runtime
    /// out of their link graph (and out of the cargo test binaries).
    pub async fn run(self, rx: flume::Receiver<ClientActionType>) {
        while let Ok(action) = rx.recv_async().await {
            self.apply(&action).await;
        }
    }

    pub async fn apply(&self, action: &ClientActionType) {
        let actions = AudioActionsManager::new(self.app_handle.clone());
        match action {
            ClientActionType::SetMuted(on) => {
                actions.set_mute(AudioDeviceType::InputDevice, *on).await;
                actions.broadcast_state().await;
            }
            ClientActionType::SetDeafened(on) => {
                actions.set_deafened(*on).await;
                actions.broadcast_state().await;
            }
            ClientActionType::SetRecording(on) => {
                if let Err(e) = actions.set_recording(*on).await {
                    warn!("ControlActionsManager: set_recording({on}) failed: {e}");
                }
                actions.broadcast_state().await;
            }
            ClientActionType::SetVolume { target, volume } => {
                // Delivered volumes drive live playback; a non-finite or
                // out-of-range gain is an ear-safety hazard regardless of source.
                if !volume.is_finite() {
                    warn!("ControlActionsManager: ignoring non-finite volume for {target}");
                    return;
                }
                self.set_gain(target, Some(volume.clamp(0.0, 1.0)), None).await;
            }
            ClientActionType::SetHeard { target, muted } => {
                self.set_gain(target, None, Some(*muted)).await;
            }
            ClientActionType::CreateGroup
            | ClientActionType::JoinGroup { .. }
            | ClientActionType::LeaveGroup => {}
        }
    }

    /// Resolves a control-action target onto the exact key the audio pipeline
    /// tracks. Preferences key on exact gamertags everywhere downstream (the
    /// sink's name→client-id remap, the dashboard's player cards), so a case-
    /// or game-prefix-variant target must land on the canonical name — never
    /// fork a ghost entry that plays no audio and renders nowhere. An exact hit
    /// wins; otherwise the first candidate matching case-insensitively with
    /// game prefixes stripped. Candidates are ordered by authority (tracked
    /// voice names before store keys). An unknown target passes through
    /// unchanged — the entry parks until that player is tracked.
    pub fn canonicalize_target(target: &str, candidates: &[&str]) -> String {
        if candidates.iter().any(|c| *c == target) {
            return target.to_string();
        }
        let norm = |s: &str| {
            s.split_once(':')
                .map(|(_, bare)| bare)
                .unwrap_or(s)
                .to_ascii_lowercase()
        };
        let wanted = norm(target);
        candidates
            .iter()
            .find(|c| norm(c) == wanted)
            .map(|c| (*c).to_string())
            .unwrap_or_else(|| target.to_string())
    }

    /// Upsert one player's gain/mute into the persisted `player_gain_store` and feed
    /// the `player_gain_store` metadata channel — the same path the dashboard UI
    /// uses (the name→client-id remap + SinkManager update happen downstream). Reads
    /// the whole store and upserts one entry, so other players' settings survive.
    async fn set_gain(&self, target: &str, volume: Option<f32>, muted: Option<bool>) {
        let store = match self.app_handle.store("store.json") {
            Ok(s) => s,
            Err(e) => {
                warn!("ControlActionsManager: store unavailable for set_gain: {e}");
                return;
            }
        };

        let mut gains: PlayerGainStore = store
            .get("player_gain_store")
            .and_then(|v| serde_json::from_value(v).ok())
            .unwrap_or_default();

        // Currently tracked voice names are the authoritative key form; the
        // existing store keys come second.
        let tracked: Vec<String> = {
            let asm = self.app_handle.state::<Mutex<AudioStreamManager>>();
            let asm = asm.lock().await;
            asm.get_current_players().into_keys().collect()
        };
        let candidates: Vec<&str> = tracked
            .iter()
            .map(String::as_str)
            .chain(gains.0.keys().map(String::as_str))
            .collect();
        let target = Self::canonicalize_target(target, &candidates);

        let entry = gains
            .0
            .entry(target.to_string())
            .or_insert(PlayerGainSettings {
                gain: 1.0,
                muted: false,
                last_seen: None,
            });
        if let Some(v) = volume {
            entry.gain = v;
        }
        if let Some(m) = muted {
            entry.muted = m;
        }

        let serialized = match serde_json::to_string(&gains) {
            Ok(s) => s,
            Err(e) => {
                warn!("ControlActionsManager: serialize player_gain_store failed: {e}");
                return;
            }
        };

        if let Ok(value) = serde_json::to_value(&gains) {
            store.set("player_gain_store", value);
            let _ = store.save();
        }

        // The dashboard's player cards read the store reactively only on its own
        // writes; nudge them so a control-plane change renders, not just plays.
        // The payload is the canonical target — the store is the source of
        // truth, but a named payload traces cleanly in device logs.
        self.app_handle
            .emit(
                crate::events::event::player_gain_store::PLAYER_GAIN_STORE_UPDATED,
                &target,
            )
            .ok();

        // The name→client-id remap that drives the SinkManager lives in the
        // OUTPUT stream's metadata handler (playback gains); the input stream
        // has no player_gain_store arm — feeding it there parks the update in a
        // cache and no audio ever changes. Same device the dashboard UI targets.
        let asm = self.app_handle.state::<Mutex<AudioStreamManager>>();
        let mut asm = asm.lock().await;
        if let Err(e) = asm
            .metadata(
                "player_gain_store".to_string(),
                serialized,
                &AudioDeviceType::OutputDevice,
            )
            .await
        {
            warn!("ControlActionsManager: player_gain_store metadata feed failed: {e}");
        }
        log::debug!(
            "ControlActionsManager: gain applied target={target} volume={volume:?} muted={muted:?}"
        );
    }
}
