use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Represents the source of how a player was added to the player store
/// This enables multi-source tracking for proximity detection vs group membership
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub enum PlayerSource {
    /// Player was detected through proximity/audio packets
    Proximity,
    /// Player was added through group/channel membership
    Group,
}

impl std::fmt::Display for PlayerSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PlayerSource::Proximity => write!(f, "proximity"),
            PlayerSource::Group => write!(f, "group"),
        }
    }
}
