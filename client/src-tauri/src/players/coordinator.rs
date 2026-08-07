use std::sync::Arc;

use common::structs::audio::{AudioDeviceType, PlayerGainSettings, PlayerGainStore};
use common::structs::players::{PlayerKey, PlayerSettingsRow};
use log::warn;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::Mutex;

use super::service::PlayerSettingsService;
use crate::audio::stream::AudioStreamManager;
use crate::structs::app_state::AppState;

/// Turns a settings change into everything that has to happen because of it.
///
/// Three effects, and dropping any one of them fails silently rather than loudly:
///
/// - the redb store, through `PlayerSettingsService`;
/// - the mixer, through the `player_gain_store` metadata channel, which is what reaches
///   `GainProjection` and is also what nudges `ControlStateBus::preferences()` so the server's
///   preference cache is refreshed;
/// - the dashboard's player cards, through `PLAYER_GAIN_STORE_UPDATED`.
///
/// `AppHandle` is taken per call rather than held as a field. A struct field whose type
/// transitively contains an `AppHandle` links the whole Tauri window and dialog destructor
/// graph into every test binary that constructs it, which aborts at load on Windows.
pub struct PlayerSettingsCoordinator {
    service: Arc<PlayerSettingsService>,
}

impl PlayerSettingsCoordinator {
    pub fn new(service: Arc<PlayerSettingsService>) -> Self {
        Self { service }
    }

    pub fn new_shared(service: Arc<PlayerSettingsService>) -> Arc<Self> {
        Arc::new(Self::new(service))
    }

    pub fn service(&self) -> &Arc<PlayerSettingsService> {
        &self.service
    }

    /// The server whose settings are in play.
    ///
    /// Resolved here rather than accepted from the webview, so a pane cannot key a row to a
    /// server this client is not connected to.
    /// `try_state` rather than `state`: the latter panics when the type is not managed, and
    /// every caller here is reachable from the webview. A command that returns an error is
    /// recoverable; a panic in a command handler is not.
    async fn current_server(app: &AppHandle) -> Result<String, anyhow::Error> {
        let state = app
            .try_state::<Mutex<AppState>>()
            .ok_or_else(|| anyhow::anyhow!("application state is not available"))?;
        let state = state.lock().await;
        state
            .current_server
            .clone()
            .ok_or_else(|| anyhow::anyhow!("no server is currently selected"))
    }

    async fn key(app: &AppHandle, cn: &str) -> Result<PlayerKey, anyhow::Error> {
        Ok(PlayerKey::new(Self::current_server(app).await?, cn))
    }

    /// This server's rows, or none when no server is selected.
    ///
    /// Empty rather than an error for the absent-server case: "nobody is signed in" is a
    /// legitimate answer to "who have I adjusted here", and the callers poll — the dashboard
    /// re-seeds its cards every ten seconds and on every foreground. Erroring would turn a
    /// signed-out window into a steady drip of failures in the log for a condition that is
    /// not one. A *mutation* with no server still errors, because there is no key to write.
    pub async fn list(&self, app: &AppHandle) -> Result<Vec<PlayerSettingsRow>, anyhow::Error> {
        match Self::current_server(app).await {
            Ok(server) => Ok(self.service.rows(&server)),
            Err(_) => Ok(Vec::new()),
        }
    }

    /// The current server's settings in the identity-keyed shape, for consumers that speak
    /// `PlayerGainStore` — the mixer feed and the control plane's preference report.
    pub async fn store_for_current_server(
        &self,
        app: &AppHandle,
    ) -> Result<PlayerGainStore, anyhow::Error> {
        Ok(self.service.store_for(&Self::current_server(app).await?))
    }

    pub async fn set_gain(&self, app: &AppHandle, cn: &str, gain: f32) -> Result<(), anyhow::Error> {
        // Out-of-range gain is an ear-safety hazard regardless of which surface asked for it.
        self.service
            .set_gain(&Self::key(app, cn).await?, gain.clamp(0.0, 2.0))?;
        self.publish(app, Some(cn)).await
    }

    pub async fn set_muted(
        &self,
        app: &AppHandle,
        cn: &str,
        muted: bool,
    ) -> Result<(), anyhow::Error> {
        self.service
            .set_muted(&Self::key(app, cn).await?, muted)?;
        self.publish(app, Some(cn)).await
    }

    pub async fn forget(&self, app: &AppHandle, cn: &str) -> Result<(), anyhow::Error> {
        self.service.forget(&Self::key(app, cn).await?)?;
        self.publish(app, Some(cn)).await
    }

    pub async fn reset_all(&self, app: &AppHandle) -> Result<(), anyhow::Error> {
        self.service.reset_all(&Self::current_server(app).await?)?;
        self.publish(app, None).await
    }

    /// Records that a player was near you, and answers with what is known about them.
    ///
    /// Does not publish: a stamp changes no volume, and proximity produces one of these per
    /// player who walks past. Returning the settings lets a caller that is rendering an
    /// arrival get both facts in one round trip — the arrival is what `last_seen` records.
    pub async fn touch(
        &self,
        app: &AppHandle,
        cn: &str,
    ) -> Result<PlayerGainSettings, anyhow::Error> {
        let key = Self::key(app, cn).await?;
        self.service.touch(&key);
        Ok(self.service.get(&key))
    }

    /// Re-seeds the mixer from managed state, for callers that have no coordinator handle.
    ///
    /// Best-effort by design: every caller is a recovery path — a device change, a stream
    /// restart, a server change — where failing to re-seed must not fail the operation itself.
    /// Must not be called while the audio stream lock is held; `publish` takes it.
    pub async fn reseed(app: &AppHandle) {
        let Some(coordinator) = app
            .try_state::<Arc<Self>>()
            .map(|state| state.inner().clone())
        else {
            return;
        };
        if let Err(cause) = coordinator.publish(app, None).await {
            warn!("PlayerSettingsCoordinator: could not re-seed the mixer: {cause}");
        }
    }

    /// Hands the current server's projection to the mixer and nudges the cards.
    ///
    /// Also the startup seed: the mixer begins with an empty projection, so until this runs
    /// once every persisted mute is inert.
    pub async fn publish(&self, app: &AppHandle, target: Option<&str>) -> Result<(), anyhow::Error> {
        let server = Self::current_server(app).await?;
        let serialized = serde_json::to_string(&self.service.store_for(&server))?;

        app.emit(
            crate::events::event::player_gain_store::PLAYER_GAIN_STORE_UPDATED,
            target.unwrap_or(server.as_str()),
        )
        .ok();

        // The OUTPUT stream's metadata handler is what reaches GainProjection. The input
        // stream has no arm for this key, so feeding it there parks the update in a cache and
        // no audio ever changes.
        let Some(manager) = app.try_state::<Mutex<AudioStreamManager>>() else {
            warn!("PlayerSettingsCoordinator: no audio stream to feed yet");
            return Ok(());
        };
        let mut manager = manager.lock().await;
        if let Err(cause) = manager
            .metadata(
                "player_gain_store".to_string(),
                serialized,
                &AudioDeviceType::OutputDevice,
            )
            .await
        {
            warn!("PlayerSettingsCoordinator: could not feed the mixer: {cause}");
        }
        Ok(())
    }
}
