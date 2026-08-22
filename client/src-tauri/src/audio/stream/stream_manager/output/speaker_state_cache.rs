use super::SpeakerState;
use common::PlayerEnum;
use moka::sync::Cache;
use std::time::Duration;

// Last known state per speaker. The server names a speaker and attaches their PlayerEnum on
// a heartbeat rather than on every frame, so most frames carry neither; this cache answers
// for the frames in between.
//
// Keyed on a string the caller derives from the envelope: a connection's device id, or the
// name of a relayed player or an injected service. One key space, because the identity and
// the position have to be evicted together whatever named them.
pub struct SpeakerStateCache {
    states: Cache<String, SpeakerState>,
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

    // The state to use for this frame.
    //
    // `named` is the speaker's name when this frame carries one, and `position` their state
    // when it carries that. The two arrive together on a heartbeat and separately elsewhere:
    // injected audio names itself on every frame but only carries a position on the
    // heartbeat, and a reduced connection frame carries neither.
    //
    // `None` means nothing has ever named this key, so the frame cannot be attributed.
    pub fn resolve(
        &self,
        key: &str,
        named: Option<String>,
        position: Option<PlayerEnum>,
    ) -> Option<SpeakerState> {
        match position {
            // A frame carrying a position refreshes the entry, which is what later frames
            // reconstruct from.
            Some(player) => {
                let name = named.or_else(|| self.states.get(key).map(|s| s.name))?;
                let state = SpeakerState {
                    name,
                    player: Some(player),
                };
                self.states.insert(key.to_string(), state.clone());
                Some(state)
            }
            // A frame carrying no position reads, and does not write: a speaker whose
            // position never arrives must still age out of the cache.
            None => {
                let cached = self.states.get(key);
                let name = named.or_else(|| cached.as_ref().map(|s| s.name.clone()))?;
                Some(SpeakerState {
                    name,
                    player: cached.and_then(|s| s.player),
                })
            }
        }
    }
}

impl Default for SpeakerStateCache {
    fn default() -> Self {
        Self::new()
    }
}
