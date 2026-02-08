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
    /// Query current state (muted, deafened, recording)
    State,
}

impl Command {
    /// Parse command from JSON text
    pub fn from_json(text: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(text)
    }
}

/// Wrapper that extracts the optional auth key alongside the command.
/// Parses key and command separately to avoid serde's flatten+tag incompatibility.
#[derive(Debug, Clone)]
pub struct CommandMessage {
    pub key: Option<String>,
    pub command: Command,
}

#[derive(Deserialize)]
struct KeyExtractor {
    key: Option<String>,
}

impl CommandMessage {
    /// Parse a full message (with optional key) from JSON text
    pub fn from_json(text: &str) -> Result<Self, serde_json::Error> {
        let key_data: KeyExtractor = serde_json::from_str(text)?;
        let command: Command = serde_json::from_str(text)?;
        Ok(Self {
            key: key_data.key,
            command,
        })
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

    #[test]
    fn test_parse_command_message_with_key() {
        let msg = CommandMessage::from_json(r#"{"action":"ping","key":"secret123"}"#).unwrap();
        assert!(matches!(msg.command, Command::Ping));
        assert_eq!(msg.key, Some("secret123".to_string()));
    }

    #[test]
    fn test_parse_command_message_without_key() {
        let msg = CommandMessage::from_json(r#"{"action":"ping"}"#).unwrap();
        assert!(matches!(msg.command, Command::Ping));
        assert_eq!(msg.key, None);
    }

    #[test]
    fn test_parse_state_command() {
        let cmd = Command::from_json(r#"{"action":"state"}"#).unwrap();
        assert!(matches!(cmd, Command::State));
    }
}
