use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// How an attempt to mint fresh Xbox Live tokens from the stored credential ended.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum BedrockRenewal {
    Renewed,
    /// The credential could not be reached, or the provider faulted. Whatever tokens are
    /// already held may still work, so the caller carries on.
    Unavailable { message: String },
    /// The credential was rejected. Only the player can fix this.
    ReauthRequired,
}

#[cfg(feature = "bedrock-protocol")]
impl From<&crate::bedrock_protocol::Error> for BedrockRenewal {
    fn from(error: &crate::bedrock_protocol::Error) -> Self {
        match error {
            crate::bedrock_protocol::Error::ReauthRequired { .. } => Self::ReauthRequired,
            other => Self::Unavailable {
                message: other.to_string(),
            },
        }
    }
}
