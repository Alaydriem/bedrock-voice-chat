use std::sync::Arc;

use tauri::async_runtime::Mutex;
use tauri::{AppHandle, Manager, Wry};
use tauri_plugin_store::Store;

use super::ControlStateBus;
use crate::structs::app_state::AppState;

/// The player's crouch-to-whisper choice.
///
/// `store.json` is the only copy. It is read each time the reporter builds a report, so the
/// settings pane and the server disagree for at most one report.
///
/// Holds the store `AppState` opened at setup rather than resolving the path. The reporter reads
/// this on a timer, and on Android resolving a store path after the activity is destroyed
/// panics.
pub struct WhisperSetting {
    store: Arc<Store<Wry>>,
}

impl WhisperSetting {
    const STORE_KEY: &'static str = "crouch_whisper_enabled";

    pub fn new(store: Arc<Store<Wry>>) -> Self {
        Self { store }
    }

    pub async fn from_app(app_handle: &AppHandle) -> Option<Self> {
        let state = app_handle.try_state::<Mutex<AppState>>()?;
        let store = state.lock().await.get_store();
        Some(Self::new(store))
    }

    /// An absent key is off. The choice is opt-in, so an install that never touched it keeps
    /// its normal range while crouching.
    pub fn enabled(&self) -> bool {
        self.store
            .get(Self::STORE_KEY)
            .and_then(|value| value.as_bool())
            .unwrap_or(false)
    }

    /// Records the choice and tells the reporter, which carries it to the server.
    pub async fn apply(app_handle: &AppHandle, enabled: bool) -> Result<bool, anyhow::Error> {
        let setting = Self::from_app(app_handle)
            .await
            .ok_or_else(|| anyhow::anyhow!("app state is not ready"))?;
        setting.store.set(Self::STORE_KEY, enabled);
        setting.store.save()?;

        if let Some(bus) = app_handle.try_state::<ControlStateBus>() {
            bus.preferences();
        }
        Ok(enabled)
    }
}
