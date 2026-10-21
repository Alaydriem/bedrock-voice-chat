use bytes::Bytes;
use serde::{Deserialize, Serialize};

use super::SocketId;

/// One step of a protected group's admission, carried in a `Direct` or `Delivered` body.
///
/// The joiner commits to its KeyPackage and nonce, a member answers with its own nonce, and the
/// joiner reveals. Both apps then derive the same 8-digit code. `check_id` names one check from
/// `Check` to `Welcome`, so a decision never applies to another joiner.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AdmissionMessage {
    /// Joiner → members. SHA-256 commitment to the KeyPackage and the joiner's nonce.
    JoinRequest { commitment: [u8; 32] },
    /// Member → joiner. The member's nonce, its current leaf signature key, the MLS group id, and
    /// the member's canonical identity, which the joiner shows by name and which proves nothing.
    Check {
        check_id: [u8; 16],
        nonce: [u8; 32],
        signature_key: Bytes,
        mls_group_id: Bytes,
        member: String,
    },
    /// Joiner → member. This commitment is already revealed, or another check is live.
    Busy { check_id: [u8; 16] },
    /// Joiner → the checking member. Opens the commitment.
    Reveal {
        check_id: [u8; 16],
        key_package: Bytes,
        nonce: [u8; 32],
    },
    /// Member → joiner, after the Add commit.
    Welcome { check_id: [u8; 16], welcome: Bytes },
    /// Member → joiner. Deny, a failed check, or an Add dropped as too old.
    Refused { check_id: [u8; 16] },
    /// Member → members, inside an MLS application message only.
    Checking { joiner: SocketId },
}

impl AdmissionMessage {
    pub fn to_bytes(&self) -> Result<Vec<u8>, anyhow::Error> {
        Ok(postcard::to_stdvec(self)?)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, anyhow::Error> {
        Ok(postcard::from_bytes(bytes)?)
    }
}
