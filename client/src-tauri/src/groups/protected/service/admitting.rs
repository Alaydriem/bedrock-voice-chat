use std::time::Instant;

use anyhow::{anyhow, bail};
use common::structs::group_key::{
    AdmissionMessage, GroupKeyFrame, GroupKeyPayload, GroupKeyRecipient, SocketId,
};
use tauri_plugin_curia::curia;

use super::{
    Clock, CodeRole, GroupKeyService, OpenRequest, PendingCommit, PendingKind,
    ProtectedGroupEvent,
};
use crate::audio::CuePolicy;
use crate::groups::protected::{JoinerRequest, MemberCheck, QueuedAdd};

impl<C: Clock> GroupKeyService<C> {
    const MAX_SHOWN_REQUESTS: usize = 5;
    const MAX_OPEN_CHECKS: usize = 3;

    pub(super) fn on_admission(&mut self, from: SocketId, message: AdmissionMessage) {
        if self.joining.is_some() {
            self.on_joiner_admission(from, message);
            return;
        }
        if self.state.is_none() {
            return;
        }
        match message {
            AdmissionMessage::Reveal {
                check_id,
                key_package,
                nonce,
            } => self.on_reveal(from, check_id, &key_package, &nonce),
            AdmissionMessage::Busy { check_id } => self.on_busy(from, check_id),
            // Only joiners receive the rest.
            _ => {}
        }
    }

    /// A join request, with the identity the server gave for the socket. One card per joiner:
    /// a repeat from the same socket or identity is dropped, not replaced.
    pub(super) fn on_join_request(&mut self, from: SocketId, identity: String, commitment: [u8; 32]) {
        if self.state.is_none() || Some(from) == self.own_socket {
            return;
        }
        if self
            .requests
            .iter()
            .any(|request| request.joiner == from || request.identity == identity)
        {
            return;
        }
        self.requests.push(OpenRequest {
            joiner: from,
            identity,
            commitment,
            received_at: self.clock.now(),
            check_id: None,
            elsewhere: false,
            admitting: false,
            shown: false,
        });
        self.show_requests();
    }

    /// The member pressed "Check code" on a card: start this member's check of that joiner.
    pub fn check(&mut self, joiner: SocketId) -> anyhow::Result<()> {
        let now = self.clock.now();
        let Some(state) = self.state.as_ref() else {
            bail!("not a member of the group");
        };
        let Some(index) = self
            .requests
            .iter()
            .position(|request| request.joiner == joiner && request.shown)
        else {
            bail!("no request from that joiner");
        };
        let request = &self.requests[index];
        if request.check_id.is_some() || request.elsewhere || request.admitting {
            bail!("that joiner is already being checked");
        }
        if self.checks.len() >= Self::MAX_OPEN_CHECKS {
            bail!("too many checks are open");
        }
        if !self.limiter.permits_check(&request.identity, now) {
            bail!("the check limit for that joiner is reached");
        }
        let (check, message) = MemberCheck::start(
            self.group_id.clone(),
            joiner,
            request.commitment,
            state.own_signature_key(),
            state.mls_group_id(),
            self.identity.to_string(),
            now,
        )?;
        let check_id = check.check_id();
        self.checks.insert(check_id, check);
        self.requests[index].check_id = Some(check_id);
        self.send_direct(joiner, message);
        self.announce_checking(joiner);
        Ok(())
    }

    /// The member pressed Admit or Deny on the card showing `check_id`. Admit queues exactly the
    /// KeyPackage that check stored; nothing is looked up by socket or name.
    pub fn decide(&mut self, joiner: SocketId, check_id: [u8; 16], allow: bool) -> anyhow::Result<()> {
        let now = self.clock.now();
        if self.checks.get(&check_id).map(MemberCheck::joiner) != Some(joiner) {
            bail!("that check is not live");
        }
        let Some(check) = self.checks.remove(&check_id) else {
            bail!("that check is not live");
        };
        let identity = self.identity_of(joiner);
        if !allow {
            if let Some(identity) = identity {
                self.limiter.record_failure(&identity, now);
            }
            self.refuse(joiner, check_id);
            self.close_request(joiner);
            return Ok(());
        }
        match check.decide(check_id, now) {
            Ok(add) => {
                if let Some(request) = self.requests.iter_mut().find(|r| r.joiner == joiner) {
                    request.check_id = None;
                    request.admitting = true;
                }
                self.queued_adds.push(add);
                self.try_commit_next_add();
                Ok(())
            }
            Err(e) => {
                self.refuse(joiner, check_id);
                self.close_request(joiner);
                Err(anyhow!("could not admit: {e:?}"))
            }
        }
    }

    /// Another member's check of `joiner`, read from an MLS application message the server
    /// cannot forge.
    pub(super) fn on_checking(&mut self, joiner: SocketId, member_leaf: u32) {
        let member_name = self
            .state
            .as_ref()
            .and_then(|state| {
                state
                    .roster()
                    .into_iter()
                    .find(|member| member.leaf == member_leaf)
            })
            .map(|member| member.identity.to_string())
            .unwrap_or_default();
        let Some(request) = self.requests.iter_mut().find(|r| r.joiner == joiner) else {
            return;
        };
        // This member is checking or admitting the joiner itself; the other check got Busy.
        // A late announcement must not take the code off this member's card.
        if request.check_id.is_some() || request.admitting {
            return;
        }
        request.elsewhere = true;
        if request.shown {
            self.emit(ProtectedGroupEvent::Checking {
                joiner,
                member_name,
            });
        }
    }

    fn on_reveal(&mut self, from: SocketId, check_id: [u8; 16], key_package: &[u8], nonce: &[u8; 32]) {
        let now = self.clock.now();
        let Some(identity) = self
            .requests
            .iter()
            .find(|request| request.check_id == Some(check_id))
            .map(|request| request.identity.clone())
        else {
            return;
        };
        let Some(check) = self.checks.get_mut(&check_id) else {
            return;
        };
        match check.on_reveal(from, check_id, key_package, nonce, &identity, &mut self.seen, now) {
            Ok(code) => self.emit(ProtectedGroupEvent::Code {
                role: CodeRole::Member,
                check_id,
                joiner: from,
                peer_name: identity,
                code: code.display(),
            }),
            Err(error) if error.is_failure() => {
                curia::info!("protected group: a check failed", {
                    channel_id: self.group_id.clone(),
                    defect: format!("{error:?}"),
                });
                self.fail_check(check_id, &identity, now);
            }
            Err(_) => {}
        }
    }

    /// A failed check ends the joiner's ticket and counts against the limit.
    fn fail_check(&mut self, check_id: [u8; 16], identity: &str, now: Instant) {
        let Some(check) = self.checks.remove(&check_id) else {
            return;
        };
        let joiner = check.joiner();
        self.limiter.record_failure(identity, now);
        self.refuse(joiner, check_id);
        self.requests.retain(|request| request.joiner != joiner);
        self.emit(ProtectedGroupEvent::CheckFailed { joiner });
        self.show_requests();
    }

    /// The joiner already revealed to someone else. Not a failure: a forged `Busy` must not burn
    /// the limit.
    fn on_busy(&mut self, from: SocketId, check_id: [u8; 16]) {
        if self.checks.get(&check_id).map(MemberCheck::joiner) != Some(from) {
            return;
        }
        self.checks.remove(&check_id);
        self.close_request(from);
    }

    /// Ends requests and checks that ran out of time. A check that expired with no reveal failed:
    /// something stopped the joiner's answer, and it counts against the limit. One the member let
    /// expire after a valid reveal is the member's choice, and does not.
    pub(super) fn expire_admissions(&mut self, now: Instant) {
        let expired: Vec<([u8; 16], SocketId, bool)> = self
            .checks
            .values()
            .filter(|check| check.is_expired(now))
            .map(|check| (check.check_id(), check.joiner(), check.is_revealed()))
            .collect();
        for (check_id, joiner, revealed) in expired {
            match (revealed, self.identity_of(joiner)) {
                (false, Some(identity)) => self.fail_check(check_id, &identity, now),
                _ => {
                    self.checks.remove(&check_id);
                    self.refuse(joiner, check_id);
                    self.close_request(joiner);
                }
            }
        }

        let stale: Vec<SocketId> = self
            .requests
            .iter()
            .filter(|request| {
                !request.admitting
                    && now.duration_since(request.received_at) >= JoinerRequest::LIFETIME
            })
            .map(|request| request.joiner)
            .collect();
        for joiner in stale {
            self.checks.retain(|_, check| check.joiner() != joiner);
            self.close_request(joiner);
        }
    }

    /// Commits the next admitted joiner, one Add at a time so each gets its own epoch. An Add
    /// too old to start is dropped: its joiner has stopped waiting.
    pub(super) fn try_commit_next_add(&mut self) {
        if self.pending.is_some() {
            return;
        }
        let now = self.clock.now();
        while !self.queued_adds.is_empty() {
            let add = self.queued_adds.remove(0);
            if add.is_too_old(now) {
                self.refuse(add.joiner, add.check_id);
                self.close_request(add.joiner);
                continue;
            }
            let Some(state) = self.state.as_mut() else {
                return;
            };
            let epoch = state.epoch();
            match state.add(&add.key_package) {
                Ok(staged) => {
                    let commit = staged.commit;
                    self.pending = Some(PendingCommit {
                        kind: PendingKind::Add {
                            add,
                            welcome: staged.welcome,
                        },
                        bytes: commit.clone(),
                        epoch,
                    });
                    self.send(GroupKeyFrame::Commit {
                        epoch,
                        body: commit.into(),
                    });
                    return;
                }
                Err(e) => {
                    curia::warn!("protected group: an admitted key package could not be added", {
                        channel_id: self.group_id.clone(),
                        error: e.to_string(),
                    });
                    self.refuse(add.joiner, add.check_id);
                    self.close_request(add.joiner);
                }
            }
        }
    }

    /// Sends the Welcome once the Add that produced it is accepted, under the id of the check that
    /// admitted the joiner.
    pub(super) fn deliver_welcome(&mut self, add: &QueuedAdd, welcome: &[u8]) {
        self.send_direct(
            add.joiner,
            AdmissionMessage::Welcome {
                check_id: add.check_id,
                welcome: welcome.to_vec().into(),
            },
        );
        self.requests.retain(|request| request.joiner != add.joiner);
        self.emit(ProtectedGroupEvent::Admitted { joiner: add.joiner });
        self.show_requests();
    }

    /// Whether a check, or an Add from one, is open. The member's leaf key must not change
    /// meanwhile: the joiner accepts a Welcome only from the key in the `Check`.
    pub(super) fn admission_open(&self) -> bool {
        !self.checks.is_empty()
            || !self.queued_adds.is_empty()
            || matches!(
                self.pending.as_ref().map(|p| &p.kind),
                Some(PendingKind::Add { .. })
            )
    }

    /// Drops every request, check and queued Add of a joiner whose socket closed.
    pub(super) fn forget_joiner(&mut self, socket: SocketId) {
        self.checks.retain(|_, check| check.joiner() != socket);
        self.queued_adds.retain(|add| add.joiner != socket);
        if matches!(
            self.pending.as_ref().map(|p| &p.kind),
            Some(PendingKind::Add { add, .. }) if add.joiner == socket
        ) {
            self.departed.insert(socket);
        }
        self.close_request(socket);
    }

    /// Removes a joiner's card and tells the webview, then raises the next waiting card.
    pub(super) fn close_request(&mut self, joiner: SocketId) {
        let before = self.requests.len();
        self.requests.retain(|request| request.joiner != joiner);
        if self.requests.len() != before {
            self.emit(ProtectedGroupEvent::RequestEnded { joiner });
            self.show_requests();
        }
    }

    /// Raises cards for requests within the shown limit, and reports how many wait beyond it.
    fn show_requests(&mut self) {
        let now = self.clock.now();
        let raised: Vec<(SocketId, String)> = self
            .requests
            .iter_mut()
            .take(Self::MAX_SHOWN_REQUESTS)
            .filter(|request| !request.shown)
            .map(|request| {
                request.shown = true;
                (request.joiner, request.identity.clone())
            })
            .collect();
        for (joiner, name) in raised {
            let cue = CuePolicy::for_join_request(self.last_cue, now).is_some();
            if cue {
                self.last_cue = Some(now);
            }
            self.emit(ProtectedGroupEvent::Request { joiner, name, cue });
        }
        let waiting = self.requests.len().saturating_sub(Self::MAX_SHOWN_REQUESTS);
        if waiting != self.waiting_reported {
            self.waiting_reported = waiting;
            self.emit(ProtectedGroupEvent::Waiting(waiting));
        }
    }

    fn announce_checking(&mut self, joiner: SocketId) {
        let Ok(data) = (AdmissionMessage::Checking { joiner }).to_bytes() else {
            return;
        };
        let Some(state) = self.state.as_mut() else {
            return;
        };
        match state.seal_application(&data) {
            Ok(message) => self.send_payload(
                GroupKeyFrame::Broadcast,
                GroupKeyPayload::Mls(message.into()),
            ),
            Err(e) => curia::warn!("protected group: could not tell the others about a check", {
                channel_id: self.group_id.clone(),
                error: e.to_string(),
            }),
        }
    }

    fn identity_of(&self, joiner: SocketId) -> Option<String> {
        self.requests
            .iter()
            .find(|request| request.joiner == joiner)
            .map(|request| request.identity.clone())
    }

    fn refuse(&self, to: SocketId, check_id: [u8; 16]) {
        self.send_direct(to, AdmissionMessage::Refused { check_id });
    }

    pub(super) fn send_direct(&self, to: SocketId, message: AdmissionMessage) {
        self.send_payload(
            |body| GroupKeyFrame::Direct {
                to: GroupKeyRecipient::Socket(to),
                body,
            },
            GroupKeyPayload::Admission(message),
        );
    }
}
