use common::PlayerEnum;
use moka::sync::Cache;
use std::time::Duration;

// Last game state seen per speaker, keyed on the envelope identity. The server attaches
// the speaker's PlayerEnum on a heartbeat rather than on every frame, so most frames
// arrive without one; this cache answers for the frames in between.
pub struct SpeakerStateCache {
    states: Cache<String, PlayerEnum>,
}

impl SpeakerStateCache {
    // A silent speaker keeps a position this long. Long enough to bridge routing gaps
    // and short mute taps; short enough that a returning speaker is not panned from
    // where they stood minutes ago.
    const IDLE_EVICTION: Duration = Duration::from_secs(30);
    const MAX_SPEAKERS: u64 = 1024;

    pub fn new() -> Self {
        Self {
            states: Cache::builder()
                .time_to_idle(Self::IDLE_EVICTION)
                .max_capacity(Self::MAX_SPEAKERS)
                .build(),
        }
    }

    // The state to use for this frame: the attached one (which also refreshes the
    // cache), or the last one seen for this speaker.
    pub fn resolve(&self, identity: &str, attached: Option<PlayerEnum>) -> Option<PlayerEnum> {
        match attached {
            Some(state) => {
                self.states.insert(identity.to_string(), state.clone());
                Some(state)
            }
            None => self.states.get(identity),
        }
    }
}

impl Default for SpeakerStateCache {
    fn default() -> Self {
        Self::new()
    }
}
