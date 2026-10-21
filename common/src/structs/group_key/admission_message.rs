use bytes::Bytes;
use serde::{Deserialize, Serialize};

use super::SocketId;

/// One step of a protected group's admission, carried in a `Direct` or `Delivered` body.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AdmissionMessage {
    PakeInit(Bytes),
    PakeReply(Bytes),
    /// The joiner's KeyPackage, with a MAC under the PAKE key over its hash.
    KeyPackageOffer { key_package: Bytes, mac: [u8; 32] },
    /// The admitter's Welcome, with a MAC under the PAKE key over its hash.
    WelcomeOffer { welcome: Bytes, mac: [u8; 32] },
    Refused,
    /// From a member that is not the group's admitter: the socket the PAKE init should go to
    /// instead. `None` when this member does not yet know the admitter's socket. Only the admitter
    /// answers PAKE inits, so its rate limit is the limit for the whole group.
    Redirect { to: Option<SocketId> },
}

impl AdmissionMessage {
    pub fn to_bytes(&self) -> Result<Vec<u8>, anyhow::Error> {
        Ok(postcard::to_stdvec(self)?)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, anyhow::Error> {
        Ok(postcard::from_bytes(bytes)?)
    }
}
