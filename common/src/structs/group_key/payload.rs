use bytes::Bytes;
use serde::{Deserialize, Serialize};

use super::AdmissionMessage;

/// What a `Direct` or `Broadcast` body carries. The relay turns both into `Delivered`, so the body
/// says which kind of message it is.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GroupKeyPayload {
    Admission(AdmissionMessage),
    /// An MLS message other than a commit: commits travel in `GroupKeyFrame::Commit`.
    Mls(Bytes),
}

impl GroupKeyPayload {
    pub fn to_bytes(&self) -> Result<Bytes, anyhow::Error> {
        Ok(Bytes::from(postcard::to_stdvec(self)?))
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, anyhow::Error> {
        Ok(postcard::from_bytes(bytes)?)
    }
}
