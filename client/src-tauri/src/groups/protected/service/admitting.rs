use common::structs::group_key::{
    AdmissionMessage, GroupKeyFrame, GroupKeyPayload, GroupKeyRecipient, SocketId,
};
use tauri_plugin_curia::curia;

use super::{Clock, GroupKeyService, PendingCommit, PendingKind};
use crate::groups::protected::{AdmitterSession, MlsGroupState};

impl<C: Clock> GroupKeyService<C> {
    pub(super) fn on_admission(&mut self, from: SocketId, message: AdmissionMessage) {
        if self.joining.is_some() {
            self.on_joiner_admission(from, message);
            return;
        }
        if self.state.is_none() {
            return;
        }
        match message {
            init @ AdmissionMessage::PakeInit(_) => self.admit_init(from, init),
            offer @ AdmissionMessage::KeyPackageOffer { .. } => self.admit_offer(from, offer),
            // Only joiners receive the rest.
            _ => {}
        }
    }

    fn admit_init(&mut self, from: SocketId, init: AdmissionMessage) {
        // Only the lowest leaf runs the PAKE. Any other member answering would keep a guess count
        // of its own, and a relay spreading guesses across members would multiply the limit.
        if let Some(admitter) = self.admitter_leaf()
            && self.state.as_ref().map(MlsGroupState::own_leaf) != Some(admitter)
        {
            self.send_direct(
                from,
                AdmissionMessage::Redirect {
                    to: self.bindings.socket_of(admitter),
                },
            );
            return;
        }

        // One admission per socket at a time. Replacing a live session would hand whoever can send
        // from that socket a fresh password guess the rate limiter never counts.
        let busy = self.admissions.contains_key(&from)
            || self.admissions.len() >= Self::MAX_CONCURRENT_ADMISSIONS
            || self.queued_adds.iter().any(|(joiner, _)| *joiner == from)
            || matches!(
                self.pending.as_ref().map(|p| &p.kind),
                Some(PendingKind::Add { joiner, .. }) if *joiner == from
            );
        if busy || !self.limiter.permits_group(self.clock.now()) {
            self.refuse(from);
            return;
        }
        match AdmitterSession::respond(&self.group_id, &self.password, init) {
            Ok((session, reply)) => {
                self.admissions.insert(from, session);
                self.send_direct(from, reply);
            }
            Err(_) => self.refuse(from),
        }
    }

    fn admit_offer(&mut self, from: SocketId, offer: AdmissionMessage) {
        let Some(session) = self.admissions.get(&from) else {
            return;
        };
        let now = self.clock.now();
        let AdmissionMessage::KeyPackageOffer { key_package, .. } = &offer else {
            return;
        };
        // A KeyPackage naming no identity could never be added, so it is refused before its MAC
        // is checked: no answer here says anything about the password.
        let Some(claimed) = MlsGroupState::key_package_identity(key_package) else {
            self.admissions.remove(&from);
            self.refuse(from);
            return;
        };
        if !self.limiter.permits(&claimed, now) {
            self.admissions.remove(&from);
            self.refuse(from);
            return;
        }
        match session.verify_offer(offer) {
            Ok(key_package) => {
                self.queued_adds.push((from, key_package));
                self.try_commit_next_add();
            }
            Err(_) => {
                self.limiter.record_failure(&claimed, now);
                self.admissions.remove(&from);
                self.refuse(from);
            }
        }
    }

    /// Commits the next verified joiner, one Add at a time so each gets its own epoch.
    pub(super) fn try_commit_next_add(&mut self) {
        if self.pending.is_some() || self.queued_adds.is_empty() {
            return;
        }
        let (joiner, key_package) = self.queued_adds.remove(0);
        let Some(state) = self.state.as_mut() else {
            return;
        };
        let epoch = state.epoch();
        match state.add(&key_package) {
            Ok(staged) => {
                self.pending = Some(PendingCommit {
                    kind: PendingKind::Add {
                        joiner,
                        key_package,
                        welcome: staged.welcome,
                    },
                    bytes: staged.commit.clone(),
                    epoch,
                });
                self.send(GroupKeyFrame::Commit {
                    epoch,
                    body: staged.commit.into(),
                });
            }
            Err(e) => {
                curia::warn!("protected group: a verified key package could not be added", {
                    channel_id: self.group_id.clone(),
                    error: e.to_string(),
                });
                self.admissions.remove(&joiner);
                self.refuse(joiner);
            }
        }
    }

    /// Sends the Welcome once the Add that produced it is accepted, bound to this admission's
    /// PAKE key so the joiner can tell it from one the relay made up.
    pub(super) fn deliver_welcome(&mut self, joiner: SocketId, welcome: &[u8]) {
        if let Some(session) = self.admissions.remove(&joiner) {
            self.send_direct(joiner, session.seal_welcome(welcome));
        }
    }

    /// The member that answers PAKE inits: the lowest leaf, which every member computes the same
    /// from state the relay cannot forge.
    fn admitter_leaf(&self) -> Option<u32> {
        self.state
            .as_ref()
            .and_then(|state| state.roster().iter().map(|m| m.leaf).min())
    }

    fn refuse(&self, to: SocketId) {
        self.send_direct(to, AdmissionMessage::Refused);
    }

    fn send_direct(&self, to: SocketId, message: AdmissionMessage) {
        self.send_payload(
            |body| GroupKeyFrame::Direct {
                to: GroupKeyRecipient::Socket(to),
                body,
            },
            GroupKeyPayload::Admission(message),
        );
    }
}
