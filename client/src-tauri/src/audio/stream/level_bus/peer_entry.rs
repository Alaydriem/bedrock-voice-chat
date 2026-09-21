use std::time::{Duration, Instant};

use common::structs::audio::ParticipantLevel;

/// One peer's last reported activity, and when it was reported.
///
/// The timestamp is the whole point. `ActivityDetector` emits only while a peer measures above
/// its threshold and never once below it, so a speaker who stops is never mentioned again.
/// Without an age, their last update — which said they were speaking — stands for the life of
/// the process, and a room where nobody is talking never reads as silent.
pub struct PeerEntry {
    level: ParticipantLevel,
    at: Instant,
}

impl PeerEntry {
    pub fn new(level: ParticipantLevel, at: Instant) -> Self {
        Self { level, at }
    }

    pub fn level(&self) -> ParticipantLevel {
        self.level
    }

    pub fn is_fresh(&self, now: Instant, ttl: Duration) -> bool {
        now.saturating_duration_since(self.at) < ttl
    }
}
