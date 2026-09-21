use std::time::{Duration, Instant};

use common::structs::audio::ParticipantLevel;

use super::LoudnessTracker;

/// One peer's loudness tracker, with the moment it was last fed.
///
/// The tracker has to be held per peer or a steady voice sitting on a step boundary flips
/// between two values every frame, and a changed value is what buys a message. Holding it
/// forever is the other failure: the map grows once per distinct speaker for the life of the
/// process, over the same names the bus already ages out.
pub struct TrackedPeer {
    tracker: LoudnessTracker,
    at: Instant,
}

impl TrackedPeer {
    pub fn new(at: Instant) -> Self {
        Self {
            tracker: LoudnessTracker::new(),
            at,
        }
    }

    pub fn observe(&mut self, rms: f32, passing: bool, now: Instant) -> ParticipantLevel {
        self.at = now;
        self.tracker.observe(rms, passing)
    }

    pub fn is_fresh(&self, now: Instant, ttl: Duration) -> bool {
        now.saturating_duration_since(self.at) < ttl
    }
}
