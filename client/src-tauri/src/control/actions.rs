use common::structs::audio::{PlayerGainSettings, PlayerGainStore};
use common::Game;
use common::structs::control::{ClientAction, ClientActionType};
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
    pub async fn run(self, rx: flume::Receiver<ClientAction>) {
        while let Ok(action) = rx.recv_async().await {
            self.apply(&action).await;
        }
    }

    pub async fn apply(&self, action: &ClientAction) {
        // The game the action was attributed with. Preference targets are keyed
        // `game:gamertag`, and a control action names a player in the actor's own game.
        let game = action.game.clone().unwrap_or(Game::Minecraft);
        let actions = AudioActionsManager::new(self.app_handle.clone());
        match &action.action {
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
                self.set_gain(target, &game, Some(volume.clamp(0.0, 1.0)), None)
                    .await;
            }
            ClientActionType::SetHeard { target, muted } => {
                self.set_gain(target, &game, None, Some(*muted)).await;
            }
            ClientActionType::CreateGroup
            | ClientActionType::JoinGroup { .. }
            | ClientActionType::LeaveGroup => {}
        }
    }

    /// Resolves a control-action target onto the exact key the audio pipeline tracks.
    ///
    /// The target arrives from a game mod as a bare in-game name, and every key downstream —
    /// the persisted gain store, the mixer's gain projection, the dashboard's player cards —
    /// is the canonical `game:gamertag`. So the target is composed against `game` first and
    /// then matched exactly.
    ///
    /// The only looseness kept is casing, and only on the gamertag: what a mod reports varies
    /// in case from what the certificate carried. The game prefix still has to match exactly,
    /// because `minecraft:Bob` and `hytale:Bob` are two people — a prefix-insensitive match
    /// would let a control action from one game mute someone in the other.
    ///
    /// Candidates are ordered by authority (tracked voice names before store keys), so the
    /// first case-insensitive hit wins. An unknown target parks under its composed canonical
    /// form rather than under the raw name, so the entry resolves once that player is tracked
    /// instead of sitting under a key nothing will ever look up.
    pub fn canonicalize_target(target: &str, game: &Game, candidates: &[&str]) -> String {
        let wanted = Self::canonical(target, game);

        if candidates.iter().any(|c| *c == wanted) {
            return wanted;
        }

        let split = |s: &str| {
            s.split_once(':')
                .map(|(tag, bare)| (tag.to_string(), bare.to_ascii_lowercase()))
        };
        let Some((wanted_tag, wanted_bare)) = split(&wanted) else {
            return wanted;
        };

        candidates
            .iter()
            .find(|c| split(c) == Some((wanted_tag.clone(), wanted_bare.clone())))
            .map(|c| (*c).to_string())
            .unwrap_or(wanted)
    }

    /// Composes a canonical identity, leaving an already-canonical name alone.
    ///
    /// The Rust counterpart of `GameNameUtils.canonical`, and idempotent for the same reason:
    /// a target can reach this from a mod (bare) or from the audio pipeline (canonical), and
    /// prefixing twice would produce `minecraft:minecraft:Bob`.
    fn canonical(name: &str, game: &Game) -> String {
        match name.split_once(':') {
            Some((tag, _)) if Game::from_tag(tag).is_some() => name.to_string(),
            _ => game.membership_key(name),
        }
    }

    /// Upsert one player's gain/mute into the persisted `player_gain_store` and feed
    /// the `player_gain_store` metadata channel — the same path the dashboard UI
    /// uses (the name→client-id remap + SinkManager update happen downstream). Reads
    /// the whole store and upserts one entry, so other players' settings survive.
    async fn set_gain(&self, target: &str, game: &Game, volume: Option<f32>, muted: Option<bool>) {
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
        let target = Self::canonicalize_target(target, game, &candidates);

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
