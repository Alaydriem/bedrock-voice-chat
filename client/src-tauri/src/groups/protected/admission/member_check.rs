use std::time::{Duration, Instant};

use bytes::Bytes;
use common::structs::group_key::{AdmissionMessage, SocketId};

use super::{
    ApprovalCode, ApprovalCommitment, CheckError, DecideError, OsEntropy, QueuedAdd,
    SeenKeyPackages,
};
use crate::groups::protected::MlsGroupState;

/// One member's check of one joiner, named by a random `check_id`.
///
/// The member picks its nonce before it sees the KeyPackage, and accepts one reveal. Admit takes
/// the exact KeyPackage stored here, for this `check_id` only, so a decision never applies to
/// another joiner.
pub struct MemberCheck {
    gid: String,
    joiner: SocketId,
    commitment: [u8; 32],
    check_id: [u8; 16],
    n_a: [u8; 32],
    signature_key: Vec<u8>,
    mls_group_id: Vec<u8>,
    started_at: Instant,
    revealed: Option<(Vec<u8>, ApprovalCode)>,
}

impl MemberCheck {
    pub const LIFETIME: Duration = Duration::from_secs(60);

    /// `own_signature_key` is the member's current leaf key; the joiner accepts a Welcome only if
    /// that key signed it. `member_name` is shown to the joiner and proves nothing.
    pub fn start(
        gid: String,
        joiner: SocketId,
        commitment: [u8; 32],
        own_signature_key: Vec<u8>,
        mls_group_id: Vec<u8>,
        member_name: String,
        now: Instant,
    ) -> anyhow::Result<(Self, AdmissionMessage)> {
        let check_id = OsEntropy::bytes::<16>()?;
        let n_a = OsEntropy::bytes::<32>()?;
        let message = AdmissionMessage::Check {
            check_id,
            nonce: n_a,
            signature_key: Bytes::copy_from_slice(&own_signature_key),
            mls_group_id: Bytes::copy_from_slice(&mls_group_id),
            member: member_name,
        };
        let check = Self {
            gid,
            joiner,
            commitment,
            check_id,
            n_a,
            signature_key: own_signature_key,
            mls_group_id,
            started_at: now,
            revealed: None,
        };
        Ok((check, message))
    }

    pub fn check_id(&self) -> [u8; 16] {
        self.check_id
    }

    pub fn joiner(&self) -> SocketId {
        self.joiner
    }

    pub fn code(&self) -> Option<ApprovalCode> {
        self.revealed.as_ref().map(|(_, code)| *code)
    }

    pub fn is_revealed(&self) -> bool {
        self.revealed.is_some()
    }

    pub fn is_expired(&self, now: Instant) -> bool {
        now.duration_since(self.started_at) >= Self::LIFETIME
    }

    /// Opens the joiner's commitment and returns the code to show. `roster_identity` is the
    /// identity the server gave for the joiner's socket; the KeyPackage must name the same one.
    #[allow(clippy::too_many_arguments)]
    pub fn on_reveal(
        &mut self,
        from: SocketId,
        check_id: [u8; 16],
        key_package: &[u8],
        n_j: &[u8; 32],
        roster_identity: &str,
        seen: &mut SeenKeyPackages,
        now: Instant,
    ) -> Result<ApprovalCode, CheckError> {
        if check_id != self.check_id {
            return Err(CheckError::WrongCheck);
        }
        if from != self.joiner {
            return Err(CheckError::WrongSender);
        }
        if self.revealed.is_some() {
            return Err(CheckError::AlreadyRevealed);
        }
        if self.is_expired(now) {
            return Err(CheckError::Expired);
        }
        if ApprovalCommitment::of(&self.gid, key_package, n_j) != Some(self.commitment) {
            return Err(CheckError::CommitmentMismatch);
        }
        let Some(identity) = MlsGroupState::validated_identity(key_package) else {
            return Err(CheckError::InvalidKeyPackage);
        };
        if !seen.record(key_package, now) {
            return Err(CheckError::Replayed);
        }
        if identity.to_string() != roster_identity {
            return Err(CheckError::IdentityMismatch);
        }
        let code = ApprovalCode::derive(
            &self.gid,
            key_package,
            n_j,
            &self.n_a,
            &self.signature_key,
            &self.mls_group_id,
        )
        .ok_or(CheckError::InvalidKeyPackage)?;
        self.revealed = Some((key_package.to_vec(), code));
        Ok(code)
    }

    /// The member pressed Admit for `check_id`. Consumes the check: its KeyPackage moves into the
    /// Add queue with its id, and a second decision for the same id finds nothing.
    pub fn decide(self, check_id: [u8; 16], now: Instant) -> Result<QueuedAdd, DecideError> {
        if check_id != self.check_id {
            return Err(DecideError::StaleCheck);
        }
        if self.is_expired(now) {
            return Err(DecideError::Expired);
        }
        let Some((key_package, _)) = self.revealed else {
            return Err(DecideError::NotRevealed);
        };
        Ok(QueuedAdd {
            check_id: self.check_id,
            joiner: self.joiner,
            key_package,
            checked_at: self.started_at,
        })
    }
}
