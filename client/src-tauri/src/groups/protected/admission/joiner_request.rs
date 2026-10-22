use std::time::{Duration, Instant};

use bytes::Bytes;
use common::structs::group_key::{AdmissionMessage, SocketId};

use super::{
    ApprovalCode, ApprovalCommitment, CheckFields, ConfirmOutcome, JoinerStage, OsEntropy,
    WelcomeRefusal,
};

/// One join request: a single-use ticket.
///
/// The joiner commits to its KeyPackage and nonce, and opens that commitment to the first member
/// that checks it and to nobody else, ever. A server that collected the opening cannot get a
/// second one to grind codes against; asking again makes a new request with a new KeyPackage.
pub struct JoinerRequest {
    gid: String,
    key_package: Vec<u8>,
    n_j: [u8; 32],
    commitment: [u8; 32],
    started_at: Instant,
    stage: JoinerStage,
}

impl JoinerRequest {
    pub const LIFETIME: Duration = Duration::from_secs(120);
    pub const CHECK_LIFETIME: Duration = Duration::from_secs(60);

    /// `gid` is the BVC channel id from the app's own state. No method takes a group id after this.
    pub fn new(
        gid: String,
        key_package: Vec<u8>,
        now: Instant,
    ) -> anyhow::Result<(Self, AdmissionMessage)> {
        let n_j = OsEntropy::bytes::<32>()?;
        let commitment = ApprovalCommitment::of(&gid, &key_package, &n_j)
            .ok_or_else(|| anyhow::anyhow!("the key package is too large to commit to"))?;
        let request = Self {
            gid,
            key_package,
            n_j,
            commitment,
            started_at: now,
            stage: JoinerStage::Requested,
        };
        Ok((request, AdmissionMessage::JoinRequest { commitment }))
    }

    pub fn commitment(&self) -> [u8; 32] {
        self.commitment
    }

    /// Opens the commitment to the first check, and returns the code. Every later check gets
    /// `Busy` naming its own id, whenever it arrives.
    pub fn on_check(
        &mut self,
        from: SocketId,
        check: CheckFields,
        now: Instant,
    ) -> Result<(AdmissionMessage, ApprovalCode), AdmissionMessage> {
        let busy = AdmissionMessage::Busy {
            check_id: check.check_id,
        };
        if !matches!(self.stage, JoinerStage::Requested) || self.is_ended(now) {
            return Err(busy);
        }
        let Some(code) = ApprovalCode::derive(
            &self.gid,
            &self.key_package,
            &self.n_j,
            &check.nonce,
            &check.signature_key,
            &check.mls_group_id,
        ) else {
            return Err(busy);
        };
        self.stage = JoinerStage::Revealed {
            check_id: check.check_id,
            checker: from,
            signature_key: check.signature_key.to_vec(),
            mls_group_id: check.mls_group_id.to_vec(),
            code,
            revealed_at: now,
            confirmed: false,
        };
        let reveal = AdmissionMessage::Reveal {
            check_id: check.check_id,
            key_package: Bytes::copy_from_slice(&self.key_package),
            nonce: self.n_j,
        };
        Ok((reveal, code))
    }

    /// The joiner's answer to the comparison. "No match" ends the request.
    pub fn confirm(&mut self, check_id: [u8; 16], matches: bool) -> ConfirmOutcome {
        let JoinerStage::Revealed {
            check_id: live,
            confirmed,
            ..
        } = &mut self.stage
        else {
            return ConfirmOutcome::Stale;
        };
        if *live != check_id {
            return ConfirmOutcome::Stale;
        }
        if matches {
            *confirmed = true;
            ConfirmOutcome::Confirmed
        } else {
            self.stage = JoinerStage::Ended;
            ConfirmOutcome::Mismatch
        }
    }

    /// Whether the joiner may enter the group a Welcome describes: the joiner confirmed this
    /// check, the Welcome came from the checking socket, its signer is the key in the `Check`,
    /// and it is for the MLS group in the `Check`.
    pub fn accept_welcome(
        &self,
        check_id: [u8; 16],
        from: SocketId,
        sender_signature_key: &[u8],
        mls_group_id: &[u8],
    ) -> Result<(), WelcomeRefusal> {
        let JoinerStage::Revealed {
            check_id: live,
            checker,
            signature_key,
            mls_group_id: expected_group,
            confirmed,
            ..
        } = &self.stage
        else {
            return Err(WelcomeRefusal::WrongCheck);
        };
        if *live != check_id || *checker != from {
            return Err(WelcomeRefusal::WrongCheck);
        }
        if !confirmed {
            return Err(WelcomeRefusal::NotConfirmed);
        }
        if signature_key.as_slice() != sender_signature_key {
            return Err(WelcomeRefusal::WrongSigner);
        }
        if expected_group.as_slice() != mls_group_id {
            return Err(WelcomeRefusal::WrongGroup);
        }
        Ok(())
    }

    /// A refusal ends the request only when it names the live check and comes from its member.
    /// Socket ids are guessable, so anyone else's refusal is ignored.
    pub fn on_refused(&mut self, from: SocketId, check_id: [u8; 16]) -> bool {
        let refused = matches!(
            &self.stage,
            JoinerStage::Revealed { check_id: live, checker, .. } if *live == check_id && *checker == from
        );
        if refused {
            self.stage = JoinerStage::Ended;
        }
        refused
    }

    /// The socket and check this request revealed to, if it has.
    pub fn checker(&self) -> Option<(SocketId, [u8; 16])> {
        match &self.stage {
            JoinerStage::Revealed {
                checker, check_id, ..
            } => Some((*checker, *check_id)),
            _ => None,
        }
    }

    pub fn is_confirmed(&self) -> bool {
        matches!(self.stage, JoinerStage::Revealed { confirmed: true, .. })
    }

    /// After 120 s; before the joiner confirms, also 60 s after the reveal.
    pub fn is_ended(&self, now: Instant) -> bool {
        if now.duration_since(self.started_at) >= Self::LIFETIME {
            return true;
        }
        match &self.stage {
            JoinerStage::Requested => false,
            JoinerStage::Revealed {
                revealed_at,
                confirmed,
                ..
            } => !confirmed && now.duration_since(*revealed_at) >= Self::CHECK_LIFETIME,
            JoinerStage::Ended => true,
        }
    }

    pub fn end(&mut self) {
        self.stage = JoinerStage::Ended;
    }
}
