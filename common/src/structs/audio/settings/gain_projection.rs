use std::collections::HashMap;
use std::sync::RwLock;

use super::player_gain::PlayerGainSettings;
use super::player_store::PlayerGainStore;

/// Per-device gain and mute, resolved from the persisted per-identity store.
///
/// Holds both inputs — the store and the device-to-identity map — behind one lock, so an answer
/// is derived at lookup instead of projected ahead of time. The projection this replaced was
/// rebuilt only when the store changed, which meant a device first heard from after the store
/// was loaded never picked up its settings. At startup that was every device, so a persisted
/// mute stayed inert until the user happened to move some other control.
///
/// Keyed on the device rather than the player because one player can speak from two devices and
/// each needs its own sink, while both read the one opinion the store holds about that player.
#[derive(Default)]
pub struct GainProjection {
    inner: RwLock<Inner>,
}

#[derive(Default)]
struct Inner {
    store: PlayerGainStore,
    devices: HashMap<u64, String>,
}

impl GainProjection {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_store(&self, store: PlayerGainStore) {
        if let Ok(mut inner) = self.inner.write() {
            inner.store = store;
        }
    }

    /// Records which player a device belongs to, from the server-stamped sender on a packet.
    ///
    /// `identity` must be canonical `game:gamertag`, because that is how the store is keyed.
    /// A bare gamertag resolves to nothing and the device plays at unity gain.
    pub fn observe(&self, device: u64, identity: &str) {
        if let Ok(mut inner) = self.inner.write() {
            inner.devices.insert(device, identity.to_string());
        }
    }

    /// Unity gain, unmuted, for a device nobody holds an opinion about. A poisoned lock returns
    /// the same default rather than panicking on the audio path.
    pub fn settings_for(&self, device: u64) -> PlayerGainSettings {
        let Ok(inner) = self.inner.read() else {
            return PlayerGainSettings::unity();
        };
        inner
            .devices
            .get(&device)
            .and_then(|identity| inner.store.0.get(identity).cloned())
            .unwrap_or_else(PlayerGainSettings::unity)
    }
}
