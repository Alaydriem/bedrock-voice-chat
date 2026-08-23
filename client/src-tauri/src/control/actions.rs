use std::sync::Arc;

use common::Game;
use common::structs::audio::PlayerGainSettings;
use common::structs::control::{ClientAction, ClientActionType};
use log::warn;
use tauri::Manager;
use tauri::async_runtime::Mutex;

use crate::audio::AudioActionsManager;
use crate::audio::AudioStreamManager;
use crate::audio::AudioDeviceType;
use crate::players::PlayerSettingsCoordinator;

/// Executes delivered `ClientAction`s against the real desktop managers. Self-state
/// actions route through `AudioActionsManager` (mute/deafen/record); per-player
/// preferences route through `PlayerSettingsCoordinator`, the same path the dashboard
/// UI and the settings pane use. Group actions are applied server-side (the client
/// learns via the existing `ChannelEvent`), so they are ignored here.
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
                self.set_gain(
                    target,
                    &game,
                    Some(volume.clamp(0.0, PlayerGainSettings::MAX_GAIN)),
                    None,
                )
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
    /// because the same gamertag under two game prefixes is two people — a prefix-insensitive
    /// match would let a control action from one game mute someone in the other.
    ///
    /// Candidates are ordered by authority (tracked voice names before store keys), so the
    /// first case-insensitive hit wins. An unknown target parks under its composed canonical
    /// form rather than under the raw name, so the entry resolves once that player is tracked
    /// instead of sitting under a key nothing will ever look up.
    pub fn canonicalize_target(target: &str, game: &Game, candidates: &[&str]) -> String {
        // The reserved jukebox target names a setting, not a player. Composing it would produce
        // `minecraft:#jukebox`, which no candidate answers for and nothing downstream reads.
        if target == common::consts::audio::JUKEBOX_CONTROL_TARGET {
            return target.to_string();
        }

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
    ///
    /// Public because channel membership is keyed the same way, so a group lookup needs it too.
    pub fn canonical(name: &str, game: &Game) -> String {
        match name.split_once(':') {
            Some((tag, _)) if Game::from_tag(tag).is_some() => name.to_string(),
            _ => game.membership_key(name).to_string(),
        }
    }

    /// Applies one player's gain or mute through the same coordinator the settings pane and
    /// the dashboard use.
    ///
    /// Routed through `PlayerSettingsCoordinator` rather than writing `store.json` directly,
    /// so the in-game panel and the desktop UI cannot hold different opinions about the same
    /// player. The coordinator owns the redb write, the mixer feed and the card nudge; the
    /// only work left here is resolving what a game mod called somebody onto the key the rest
    /// of the client uses.
    async fn set_gain(&self, target: &str, game: &Game, volume: Option<f32>, muted: Option<bool>) {
        if target == common::consts::audio::JUKEBOX_CONTROL_TARGET {
            self.set_jukebox(volume, muted).await;
            return;
        }

        let Some(coordinator) = self
            .app_handle
            .try_state::<Arc<PlayerSettingsCoordinator>>()
            .map(|state| state.inner().clone())
        else {
            warn!("ControlActionsManager: player settings unavailable for set_gain");
            return;
        };

        // Currently tracked voice names are the authoritative key form; the keys already in
        // the store come second.
        // Scoped so the guard is released before `coordinator.list` below, which takes the
        // AppState lock. Nothing may hold the audio stream lock and then wait on AppState:
        // `set_audio_device` already holds AppState while waiting on this one, so the reverse
        // edge would complete a deadlock cycle.
        let tracked: Vec<String> = {
            let Some(asm) = self.app_handle.try_state::<Mutex<AudioStreamManager>>() else {
                warn!("ControlActionsManager: no audio stream for set_gain");
                return;
            };
            let asm = asm.lock().await;
            asm.get_current_players().into_keys().collect()
        };
        let known: Vec<String> = coordinator
            .list(&self.app_handle)
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|row| row.key.cn)
            .collect();
        let candidates: Vec<&str> = tracked
            .iter()
            .map(String::as_str)
            .chain(known.iter().map(String::as_str))
            .collect();
        let target = Self::canonicalize_target(target, game, &candidates);

        if let Some(v) = volume {
            if let Err(e) = coordinator.set_gain(&self.app_handle, &target, v).await {
                warn!("ControlActionsManager: set_gain failed for {target}: {e}");
                return;
            }
        }
        if let Some(m) = muted {
            if let Err(e) = coordinator.set_muted(&self.app_handle, &target, m).await {
                warn!("ControlActionsManager: set_muted failed for {target}: {e}");
                return;
            }
        }

        log::debug!(
            "ControlActionsManager: gain applied target={target} volume={volume:?} muted={muted:?}"
        );
    }

    /// Applies a jukebox action through the backend's own single write path.
    ///
    /// Diverted before the coordinator is reached, so nothing about the jukebox ever enters the
    /// per-player gain store — that store is what the dashboard builds player cards from, and an
    /// entry there would render the jukebox as a person.
    async fn set_jukebox(&self, volume: Option<f32>, muted: Option<bool>) {
        let actions = AudioActionsManager::new(self.app_handle.clone());

        if let Some(gain) = volume {
            if let Err(e) = actions.set_jukebox_gain(gain).await {
                warn!("ControlActionsManager: set_jukebox_gain failed: {e}");
                return;
            }
        }
        if let Some(m) = muted {
            if let Err(e) = actions.set_jukebox_muted(m).await {
                warn!("ControlActionsManager: set_jukebox_muted failed: {e}");
                return;
            }
        }

        log::debug!("ControlActionsManager: jukebox applied volume={volume:?} muted={muted:?}");
    }
}
