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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn test_player_source_serialization() {
        let proximity = PlayerSource::Proximity;
        let group = PlayerSource::Group;
        
        // Test JSON serialization
        let proximity_json = serde_json::to_string(&proximity).unwrap();
        let group_json = serde_json::to_string(&group).unwrap();
        
        assert_eq!(proximity_json, "\"Proximity\"");
        assert_eq!(group_json, "\"Group\"");
        
        // Test deserialization
        let proximity_back: PlayerSource = serde_json::from_str(&proximity_json).unwrap();
        let group_back: PlayerSource = serde_json::from_str(&group_json).unwrap();
        
        assert_eq!(proximity_back, PlayerSource::Proximity);
        assert_eq!(group_back, PlayerSource::Group);
    }
    
    #[test]
    fn test_player_source_in_set() {
        let mut sources = HashSet::new();
        sources.insert(PlayerSource::Proximity);
        sources.insert(PlayerSource::Group);
        
        assert!(sources.contains(&PlayerSource::Proximity));
        assert!(sources.contains(&PlayerSource::Group));
        assert_eq!(sources.len(), 2);
        
        sources.remove(&PlayerSource::Proximity);
        assert!(!sources.contains(&PlayerSource::Proximity));
        assert!(sources.contains(&PlayerSource::Group));
        assert_eq!(sources.len(), 1);
    }
    
    #[test]
    fn test_player_source_display() {
        assert_eq!(PlayerSource::Proximity.to_string(), "proximity");
        assert_eq!(PlayerSource::Group.to_string(), "group");
    }
}