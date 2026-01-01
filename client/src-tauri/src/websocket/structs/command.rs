use serde::{Deserialize, Serialize};
use schemars::JsonSchema;
use super::DeviceType;

/// Base command structure with action discriminator
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "action", rename_all = "lowercase")]
pub enum Command {
    /// Health check command - returns pong response
    Ping,
    /// Toggle mute for input or output device - returns current mute status
    Mute { device: DeviceType },
    /// Toggle recording on/off - returns current recording status
    Record,
}

impl Command {
    /// Parse command from JSON text
    pub fn from_json(text: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ping() {
        let cmd = Command::from_json(r#"{"action":"ping"}"#).unwrap();
        assert!(matches!(cmd, Command::Ping));
    }

    #[test]
    fn test_parse_mute() {
        let cmd = Command::from_json(r#"{"action":"mute","device":"input"}"#).unwrap();
        match cmd {
            Command::Mute { device } => assert!(matches!(device, DeviceType::Input)),
            _ => panic!("Expected Mute command"),
        }
    }

    #[test]
    fn test_parse_record() {
        let cmd = Command::from_json(r#"{"action":"record"}"#).unwrap();
        assert!(matches!(cmd, Command::Record));
    }
}
