use std::sync::Mutex;

use super::DeviceSnapshot;
use crate::audio::types::AudioDeviceType;
use crate::structs::StoredAudioDevice;


// Cached device and mute state.
//
// Reading a device goes through the app state, which is also held by the command path and whose
// input accessor lazily initializes behind a permission check. Doing that once per second would
// contend with commands for no benefit — a device name does not change at that rate — so the
// values are refreshed on a slow cadence and served from cache in between.
//
// A refresh that cannot take the lock leaves the previous values in place. A stale device name
// is a cosmetic inaccuracy; blocking a diagnostic tick on a contended lock is not.
#[derive(Debug, Default)]
pub struct DeviceInfo {
    cached: Mutex<DeviceSnapshot>,
}

impl DeviceInfo {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn snapshot(&self) -> DeviceSnapshot {
        self.cached
            .lock()
            .map(|g| g.clone())
            .unwrap_or_default()
    }

    // Devices are read from the persisted store rather than through `AppState::get_audio_device`.
    // That accessor performs a microphone permission check and lazily initializes the device, so
    // calling it here would panic in any context where the permissions plugin is not registered —
    // and a diagnostic tick must never be able to bring down a worker.
    pub fn refresh(&self, app_handle: &tauri::AppHandle) {
        let family_preference =
            tauri::Manager::try_state::<tauri::async_runtime::Mutex<crate::AppState>>(app_handle)
                .and_then(|state| state.try_lock().ok().map(|s| s.family_preference().get()));

        let input = Self::device_from_store(app_handle, AudioDeviceType::InputDevice);
        let output = Self::device_from_store(app_handle, AudioDeviceType::OutputDevice);
        let muted_peer_count = Self::muted_peer_count(app_handle);

        if let Ok(mut cached) = self.cached.lock() {
            if let Some((name, rate)) = input {
                cached.input_name = Some(name);
                cached.input_sample_rate = rate;
            }
            if let Some((name, rate)) = output {
                cached.output_name = Some(name);
                cached.output_sample_rate = rate;
            }
            if let Some(count) = muted_peer_count {
                cached.muted_peer_count = count;
            }
            if let Some(preference) = family_preference {
                cached.family_preference = Some(preference);
            }
        }
    }

    // Test-only passthrough to the process-global flags, so an integration test can prove the
    // snapshot tracks them rather than reporting a default.
    #[cfg(any(test, feature = "e2e"))]
    pub fn set_mute_state_for_test(muted: bool, deafened: bool) {
        use crate::audio::stream::stream_manager::MuteFlags;
        MuteFlags::set_input_muted(muted);
        MuteFlags::set_output_muted(deafened);
    }

    // `None` until the user has chosen a device, in which case the app is on the system default and
    // there is no stored name to report.
    //
    // Goes through `StoredAudioDevice` rather than reading the JSON by hand so this and the startup
    // path cannot disagree about the stored shape — notably that stream configs sit under `config`,
    // not `stream_configs`.
    fn device_from_store(
        app_handle: &tauri::AppHandle,
        io: AudioDeviceType,
    ) -> Option<(String, Option<u32>)> {
        let store = tauri_plugin_store::StoreExt::store(app_handle, "store.json").ok()?;
        let stored = StoredAudioDevice::peek(io, &store)?;

        Some((
            stored.display_name().to_string(),
            stored.best_sample_rate(),
        ))
    }

    // Read live rather than cached. Mute toggles constantly and from several places — a keybind,
    // the UI, an in-game command, a WebSocket client — so a value refreshed every thirty seconds
    // would contradict what the user just did.
    pub fn mute_state() -> (bool, bool) {
        use crate::audio::stream::stream_manager::MuteFlags;
        (MuteFlags::input_muted(), MuteFlags::output_muted())
    }

    /// Whether the noise gate is bound to the capture path.
    ///
    /// Read live, from the flag the capture path itself consults. Reading the settings
    /// store instead would report the copy the user set rather than the one the audio
    /// thread obeys, and a disagreement between those two is the whole reason this is
    /// worth reporting.
    pub fn noise_gate_enabled() -> bool {
        use crate::audio::stream::stream_manager::NoiseGateFlags;
        NoiseGateFlags::enabled()
    }

    // Test-only setter, for the same reason as the mute ones: the flag is process-global and
    // normally moved by the settings screen, so without this a test can only observe the
    // default and cannot tell a wired field from a hardcoded one.
    #[cfg(any(test, feature = "e2e"))]
    pub fn set_noise_gate_enabled(enabled: bool) {
        use crate::audio::stream::stream_manager::NoiseGateFlags;
        NoiseGateFlags::set_enabled(enabled);
    }

    /// How many players on the current server the user has muted.
    ///
    /// `try_lock` rather than a blocking lock, matching `family_preference` above: this runs
    /// on the diagnostics refresh, and a momentarily contended lock is worth a stale count
    /// rather than a stalled report.
    /// How many players on the current server the user has muted, or `None` when the answer
    /// cannot be read right now.
    ///
    /// `None` rather than `0` on a contended lock, because the caller keeps the previous value
    /// instead of overwriting it — the same treatment `family_preference` gets. Reporting a
    /// confident zero every time the settings pane happens to hold the lock would make the
    /// diagnostics say "you have muted nobody" to a user who has muted several people, which
    /// is exactly the kind of wrong that sends support down the wrong path.
    fn muted_peer_count(app_handle: &tauri::AppHandle) -> Option<u32> {
        let players = tauri::Manager::try_state::<
            std::sync::Arc<crate::players::PlayerSettingsService>,
        >(app_handle)?;

        let server = tauri::Manager::try_state::<tauri::async_runtime::Mutex<crate::AppState>>(
            app_handle,
        )?
        .try_lock()
        .ok()?
        .current_server
        .clone()?;

        Some(players.muted_count(&server))
    }
}
