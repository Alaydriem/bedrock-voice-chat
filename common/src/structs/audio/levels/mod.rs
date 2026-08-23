mod participant;

pub use participant::ParticipantLevel;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use ts_rs::TS;

/// Everyone's voice activity in one message.
///
/// One message rather than two. The self level and the peer levels were separate events on
/// separate timers, each costing a webview delivery of its own, and on Android every delivery
/// is a unit of main-thread work — so two streams of them cost twice as much main thread for
/// information that is always read together.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct LevelSnapshot {
    /// This client's own microphone.
    pub own: ParticipantLevel,
    /// Everyone audible, keyed by the name the roster uses.
    pub peers: HashMap<String, ParticipantLevel>,
}

impl LevelSnapshot {
    pub fn silent() -> Self {
        Self {
            own: ParticipantLevel::silent(),
            peers: HashMap::new(),
        }
    }

    /// Whether anybody is producing audio.
    ///
    /// A snapshot where this is false twice running is worth no message at all: the client
    /// decays its own meters to rest, so silence needs nothing sent to represent it.
    pub fn is_silent(&self) -> bool {
        !self.own.speaking && !self.peers.values().any(|p| p.speaking)
    }

    /// Whether anyone's state changed in a way a viewer could see.
    ///
    /// A peer disappearing counts: their meter has to be told to stop.
    pub fn differs_from(&self, other: &Self) -> bool {
        if self.own.differs_from(&other.own) || self.peers.len() != other.peers.len() {
            return true;
        }

        self.peers.iter().any(|(name, level)| {
            other
                .peers
                .get(name)
                .is_none_or(|previous| level.differs_from(previous))
        })
    }
}
