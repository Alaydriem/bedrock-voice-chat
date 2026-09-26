use super::SpeakerState;
use common::structs::packet::SpeakerPosition;
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
        position: Option<SpeakerPosition>,
    ) -> Option<SpeakerState> {
        match position {
            // A frame carrying a position refreshes the entry, which is what later frames
            // reconstruct from.
            Some(speaker) => {
                let name = named.or_else(|| self.states.get(key).map(|s| s.name))?;
                let state = SpeakerState {
                    name,
                    speaker: Some(speaker),
                };
                self.states.insert(key.to_string(), state.clone());
                Some(state)
            }
            // A frame carrying no position never overwrites a cached one — the last known
            // position is the whole reason later frames can be placed. It does record a name
            // this key had none for: a speaker with no position is named by the attach
            // heartbeat and by no frame in between, so without this the frames in between
            // resolve to nothing and are discarded, and a group member who has not reported a
            // position is heard one frame in eight.
            //
            // Aging is unaffected. `time_to_idle` is refreshed by the read below either way,
            // so an entry still lapses once the speaker stops sending.
            None => {
                let cached = self.states.get(key);
                let name = named.or_else(|| cached.as_ref().map(|s| s.name.clone()))?;
                let state = SpeakerState {
                    name,
                    speaker: cached.as_ref().and_then(|s| s.speaker.clone()),
                };
                if cached.is_none() {
                    self.states.insert(key.to_string(), state.clone());
                }
                Some(state)
            }
        }
    }
}

impl Default for SpeakerStateCache {
    fn default() -> Self {
        Self::new()
    }
}
