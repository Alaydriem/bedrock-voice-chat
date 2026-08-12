use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::DeviceType;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "action", rename_all = "lowercase")]
pub enum Command {
    Ping,
    Mute { device: DeviceType },
    Record,
    /// Toggle whether jukebox music plays.
    ///
    /// A toggle rather than a setter, matching `Mute`: a controller button cannot read the
    /// current state before it is pressed.
    Jukebox,
    /// Set how loud jukebox music plays, as a percent where 100 is untouched.
    ///
    /// A level rather than a toggle, unlike `Jukebox`: a dial sends an absolute value and knows
    /// what it is asking for. A level above the ceiling is clamped rather than refused, so a
    /// controller with a wider dial still lands at the loudest the client will play.
    JukeboxVolume { level: u8 },
    State,
    /// Push-to-talk, held. `down` is the press; the mic closes on the release.
    ///
    /// A controller holding this open is responsible for sending the release. A dropped
    /// connection does not close the mic, so a button that latches leaves it open.
    Ptt { down: bool },
    /// The worlds this client can be asked to connect to.
    Targets,
    /// Connect to one of them. `id` comes from a prior `targets` response.
    Connect { id: String },
    /// Stop whichever session is live.
    ///
    /// Names no target. Idempotent, so a controller that lost track of the client can send
    /// it without first asking what is running.
    Disconnect,
    /// Create a group with this name and join it.
    ///
    /// A name that already exists is created anyway: the caller named a create.
    CreateGroup { name: String },
    /// Join the existing group with this name.
    ///
    /// Refused when the name matches no group, and when it matches more than one.
    JoinGroup { name: String },
    /// Leave whichever group this client is in.
    ///
    /// Names nothing, and is idempotent, so a controller that lost track can send it without
    /// first asking what it is in.
    LeaveGroup,
}

impl Command {
    pub fn from_json(text: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(text)
    }
}

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
    fn test_parse_ptt() {
        let cmd = Command::from_json(r#"{"action":"ptt","down":true}"#).unwrap();
        match cmd {
            Command::Ptt { down } => assert!(down),
            _ => panic!("Expected Ptt command"),
        }
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
