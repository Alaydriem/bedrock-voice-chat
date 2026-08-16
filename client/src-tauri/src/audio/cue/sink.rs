use crate::audio::cue::Cue;
use rodio::buffer::SamplesBuffer;
use rodio::mixer::Mixer;
use std::sync::{Arc, RwLock};

/// Plays cues onto whichever mixer the current session built.
///
/// Onto the mixer rather than through a sink of its own for two reasons. The global mute
/// that deafen sets is a multiplier on each player's sink, applied inside the sink loop —
/// so a source added at the mixer is still heard, which is what makes an audible deafen cue
/// possible at all. And a short-lived stream of its own would have to open the output device
/// on every keypress: an exclusive-mode ASIO device is already held by the session and would
/// simply refuse.
///
/// The mixer is re-read on every play rather than held by the caller. An output-stream
/// rebuild — a device change, a capture watchdog recovery — replaces it, and a cached handle
/// would go silent for the rest of the session with nothing to show for it.
pub struct CueSink {
    target: RwLock<Option<(Arc<Mixer>, u32, u16)>>,

    #[cfg(any(test, feature = "e2e"))]
    played: std::sync::Mutex<Vec<Cue>>,
}

impl CueSink {
    pub fn new() -> Self {
        Self {
            target: RwLock::new(None),

            #[cfg(any(test, feature = "e2e"))]
            played: std::sync::Mutex::new(Vec::new()),
        }
    }

    pub fn new_shared() -> Arc<Self> {
        Arc::new(Self::new())
    }

    /// Adopt the mixer a freshly built output stream produced, with the rate and channel
    /// count it runs at so nothing resamples on the way out.
    pub fn attach(&self, mixer: Arc<Mixer>, sample_rate: u32, channels: u16) {
        if let Ok(mut target) = self.target.write() {
            *target = Some((mixer, sample_rate, channels));
        }
    }

    /// Play a cue, or do nothing where no session has built a mixer yet.
    ///
    /// Silence rather than an error: mute is reachable before a session is up, and a cue is
    /// a courtesy. Failing the mute because the tone could not play would be worse than the
    /// missing tone.
    pub fn play(&self, cue: Cue) {
        #[cfg(any(test, feature = "e2e"))]
        if let Ok(mut played) = self.played.lock() {
            played.push(cue);
        }

        let Ok(target) = self.target.read() else {
            return;
        };
        let Some((mixer, sample_rate, channels)) = target.as_ref() else {
            return;
        };

        let (Some(buffer_channels), Some(buffer_rate)) = (
            std::num::NonZeroU16::new(*channels),
            std::num::NonZeroU32::new(*sample_rate),
        ) else {
            return;
        };

        let samples = cue.samples(*sample_rate, *channels);
        mixer.add(SamplesBuffer::new(buffer_channels, buffer_rate, samples));
    }

    /// Which cues this session decided to play, in order.
    ///
    /// The observable seam for the emission rules. Whether a tone reached a speaker is not
    /// something a test can see; which cue the app chose is, and that is where the bugs are.
    #[cfg(any(test, feature = "e2e"))]
    pub fn played(&self) -> Vec<Cue> {
        self.played
            .lock()
            .map(|played| played.clone())
            .unwrap_or_default()
    }
}

impl Default for CueSink {
    fn default() -> Self {
        Self::new()
    }
}
