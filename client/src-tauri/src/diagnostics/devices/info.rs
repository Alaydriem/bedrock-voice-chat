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
            cached.muted_peer_count = muted_peer_count;
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

    fn muted_peer_count(app_handle: &tauri::AppHandle) -> u32 {
        let Some(store) = tauri_plugin_store::StoreExt::store(app_handle, "store.json").ok() else {
            return 0;
        };

        let Some(value) = store.get("player_gain_store") else {
            return 0;
        };

        let Some(map) = value.as_object() else {
            return 0;
        };

        map.values()
            .filter(|entry| {
                entry
                    .get("muted")
                    .and_then(|m| m.as_bool())
                    .unwrap_or(false)
            })
            .count() as u32
    }
}
