use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// The voice mode, as it appears on the wire.
///
/// A local mirror of the client's own enum, like `DeviceType`: this crate is the protocol
/// schema an external controller compiles against, and it does not carry the client's
/// dependencies to get one enum.
///
/// It decides what the mute control means. In `openMic` the mic button is a toggle; in
/// `pushToTalk` it is a hold, and a toggle there would be a second word for a state the
/// hold already owns.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum VoiceMode {
    #[serde(rename = "openMic")]
    OpenMic,
    #[serde(rename = "pushToTalk")]
    PushToTalk,
}
