use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use super::player_gain::PlayerGainSettings;

/// The one opinion a player holds about jukebox music, applied to every jukebox sink.
///
/// Atomics rather than a lock because this is read once per frame on the mixing path and
/// written only when somebody moves a control.
///
/// Answering per sink key is what keeps concurrent playbacks independent: the caller asks once
/// for each sink it is mixing, so any number of jukeboxes each resolve their own volume at
/// their own position from the one setting held here.
pub struct JukeboxLevel {
    gain: AtomicU32,
    muted: AtomicBool,
}

impl Default for JukeboxLevel {
    fn default() -> Self {
        Self {
            gain: AtomicU32::new(1.0f32.to_bits()),
            muted: AtomicBool::new(false),
        }
    }
}

impl JukeboxLevel {
    /// The top of the slider. The same ceiling as a peer's voice, so 100% means untouched
    /// everywhere and one number governs every gain in the product.
    pub const MAX_GAIN: f32 = PlayerGainSettings::MAX_GAIN;

    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_gain(&self, gain: f32) {
        self.gain
            .store(gain.clamp(0.0, Self::MAX_GAIN).to_bits(), Ordering::Relaxed);
    }

    pub fn set_muted(&self, muted: bool) {
        self.muted.store(muted, Ordering::Relaxed);
    }

    pub fn gain(&self) -> f32 {
        f32::from_bits(self.gain.load(Ordering::Relaxed))
    }

    pub fn is_muted(&self) -> bool {
        self.muted.load(Ordering::Relaxed)
    }

    /// What a sink filed under `sink_key` should play at.
    ///
    /// Only a jukebox key is answered for. Channel API audio is synthetic as well and reaches
    /// the caller's same arm, and it takes unity so an announcement is never silenced by a
    /// music control.
    pub fn settings_for(&self, sink_key: &str) -> PlayerGainSettings {
        if !sink_key.starts_with(crate::consts::audio::JUKEBOX_PLAYER_PREFIX) {
            return PlayerGainSettings::unity();
        }

        PlayerGainSettings {
            gain: self.gain(),
            muted: self.is_muted(),
            last_seen: None,
        }
    }
}
