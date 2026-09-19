use crate::audio::cue::{Cue, CueSink};
use std::sync::Arc;
use tauri::{AppHandle, Manager};
use tauri_plugin_store::StoreExt;

/// Plays a cue if the user still wants to hear cues.
///
/// Borrowed rather than owned because both callers already hold the handle: the actions
/// manager that drives mute, and the packet router that sees the server's channel events.
/// Two copies of the flag rule is how one surface keeps announcing itself after the switch
/// is turned off.
pub struct CueAnnouncer<'a> {
    app_handle: &'a AppHandle,
}

impl<'a> CueAnnouncer<'a> {
    pub fn new(app_handle: &'a AppHandle) -> Self {
        Self { app_handle }
    }

    /// Whether voice chat announces itself.
    ///
    /// Read from the store on each change rather than cached. The plugin keeps the file in
    /// memory so this is a map lookup, and one copy cannot drift from the settings pane the
    /// way a mirrored flag would.
    ///
    /// An absent key is on. Every install that predates this feature has no key, and reading
    /// that as off would ship the feature switched off for everyone who already has BVC.
    fn enabled(&self) -> bool {
        self.app_handle
            .store("store.json")
            .ok()
            .and_then(|store| store.get("mute_cues_enabled"))
            .and_then(|value| value.as_bool())
            .unwrap_or(true)
    }

    pub fn play(&self, cue: Cue) {
        if !self.enabled() {
            return;
        }
        if let Some(sink) = self.app_handle.try_state::<Arc<CueSink>>() {
            sink.play(cue);
        }
    }
}
