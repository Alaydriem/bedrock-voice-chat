use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Why a send from the app did not go through.
///
/// Shared rather than server-local because the sender is told: a refusal that only the server
/// knows about is indistinguishable, from the composer, from a message that landed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS, thiserror::Error)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub enum ChatRejection {
    #[error("no chat channel is registered for this world")]
    NoChannel,

    /// The sender is in game somewhere other than the world they addressed — they were
    /// transferred while the app still held the older target. Delivering anyway would put
    /// their message in front of people they are not standing with.
    #[error("sender is not in this world")]
    WrongWorld { current: Option<String> },

    /// Named no world at all, so there was nothing to address.
    #[error("no world was named")]
    NoWorld,
}
