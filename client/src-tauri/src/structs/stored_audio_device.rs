use std::sync::Arc;

use serde::Deserialize;
use tauri::Wry;
use tauri_plugin_store::Store;

use crate::audio::types::{AudioDeviceHost, AudioDeviceType, StreamConfig};

// The persisted form of a selected audio device.
//
// The stored shape predates `AudioDevice` and does not match it field for field: stream configs are
// held under `config` rather than `stream_configs`, and `display_name` is absent on entries written
// by older builds. Deserializing through one type keeps that mapping in a single place, so the
// startup path and the diagnostics reader cannot disagree about where the sample rate lives.
#[derive(Debug, Clone, Deserialize)]
pub struct StoredAudioDevice {
    pub id: String,
    pub name: String,
    pub host: AudioDeviceHost,
    #[serde(rename = "config")]
    pub stream_configs: Vec<StreamConfig>,
    #[serde(default)]
    pub display_name: Option<String>,
}

impl StoredAudioDevice {
    /// Reads the persisted device for `io`, or `None` to mean "fall back to the system default".
    ///
    /// Every unreadable entry is discarded rather than reported as an error. This runs during app
    /// setup and again from the audio command path, so a caller that unwrapped a failure here would
    /// take the app down before the user could reach the UI to select a different device — leaving
    /// no route to recovery from a single bad key in `store.json`.
    ///
    /// Entries written before the `id` field existed surface here as an ordinary deserialization
    /// failure naming that field, which is why they need no separate check.
    pub fn load(io: AudioDeviceType, store: &Arc<Store<Wry>>) -> Option<Self> {
        let raw = store.get(io.store_key())?;

        match serde_json::from_value::<Self>(raw) {
            Ok(stored) if stored.stream_configs.is_empty() => {
                log::warn!(
                    "Stored device config for {} lists no stream configurations; reverting to the system default.",
                    io.store_key()
                );
                Self::discard(io, store);
                None
            }
            Ok(stored) => Some(stored),
            Err(e) => {
                log::warn!(
                    "Discarding unreadable device config for {}: {}. Reverting to the system default.",
                    io.store_key(),
                    e
                );
                Self::discard(io, store);
                None
            }
        }
    }

    /// Reads the persisted device without repairing or logging anything.
    ///
    /// For observers — a diagnostic tick reports what is stored and must not rewrite it. `load` is
    /// for the paths that are about to act on the value and therefore have to resolve a bad entry.
    pub fn peek(io: AudioDeviceType, store: &Arc<Store<Wry>>) -> Option<Self> {
        let raw = store.get(io.store_key())?;

        serde_json::from_value::<Self>(raw)
            .ok()
            .filter(|stored| !stored.stream_configs.is_empty())
    }

    /// Falls back to `name` for entries predating `display_name`.
    pub fn display_name(&self) -> &str {
        self.display_name.as_deref().unwrap_or(&self.name)
    }

    /// The highest rate this device advertised when it was stored.
    pub fn best_sample_rate(&self) -> Option<u32> {
        self.stream_configs.iter().map(|c| c.sample_rate).max()
    }

    fn discard(io: AudioDeviceType, store: &Arc<Store<Wry>>) {
        store.delete(io.store_key());
        let _ = store.save();
    }
}
