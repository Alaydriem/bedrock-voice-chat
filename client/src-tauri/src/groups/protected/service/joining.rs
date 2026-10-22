use anyhow::bail;
use common::structs::group_key::{
    AdmissionMessage, GroupKeyFrame, GroupKeyPayload, GroupKeyRecipient, SocketId,
};
use tauri_plugin_curia::curia;

use super::{
    Clock, CodeRole, GroupKeyService, HeldWelcome, JoinPhase, LostReason, ProtectedGroupEvent,
};
use crate::groups::protected::{CheckFields, ConfirmOutcome, JoinerRequest, MlsGroupState};

impl<C: Clock> GroupKeyService<C> {
    /// Sends the join request to every member at once: a commitment to a fresh KeyPackage, which
    /// the first member to check it gets opened to and nobody else.
    pub(super) fn start_request(&mut self) {
        if !matches!(
            self.joining.as_ref().map(|j| &j.phase),
            Some(JoinPhase::Waiting)
        ) {
            return;
        }
        let (pending, key_package) = match MlsGroupState::key_package(&self.identity) {
            Ok(material) => material,
            Err(e) => {
                curia::error!("protected group: could not build a key package", {
                    channel_id: self.group_id.clone(),
                    error: e.to_string(),
                });
                self.lose(LostReason::ProtocolFailure);
                return;
            }
        };
        let now = self.clock.now();
        let (request, message) = match JoinerRequest::new(self.group_id.clone(), key_package, now)
        {
            Ok(request) => request,
            Err(e) => {
                curia::error!("protected group: could not start a join request", {
                    channel_id: self.group_id.clone(),
                    error: e.to_string(),
                });
                self.lose(LostReason::ProtocolFailure);
                return;
            }
        };
        if let Some(joining) = self.joining.as_mut() {
            joining.phase = JoinPhase::Asking {
                request,
                pending,
                held: None,
            };
        }
        self.send_payload(
            |body| GroupKeyFrame::Direct {
                to: GroupKeyRecipient::Members,
                body,
            },
            GroupKeyPayload::Admission(message),
        );
    }

    pub(super) fn tick_joining(&mut self) {
        let now = self.clock.now();
        if self.request().is_some_and(|request| request.is_ended(now)) {
            self.lose(LostReason::Expired);
        }
    }

    pub(super) fn on_joiner_admission(&mut self, from: SocketId, message: AdmissionMessage) {
        match message {
            AdmissionMessage::Check {
                check_id,
                nonce,
                signature_key,
                mls_group_id,
                member,
            } => self.on_check(
                from,
                CheckFields {
                    check_id,
                    nonce,
                    signature_key,
                    mls_group_id,
                    member,
                },
            ),
            AdmissionMessage::Welcome { check_id, welcome } => {
                self.on_welcome(from, check_id, welcome.to_vec());
            }
            AdmissionMessage::Refused { check_id } => {
                if self
                    .request_mut()
                    .is_some_and(|request| request.on_refused(from, check_id))
                {
                    self.lose(LostReason::Refused);
                }
            }
            // A joiner receives nothing else; anything else is not this join's answer.
            _ => {}
        }
    }

    /// The joiner's answer to the comparison: "It matches" lets a held Welcome in, "No match"
    /// ends the request.
    pub fn confirm(&mut self, check_id: [u8; 16], matches: bool) -> anyhow::Result<()> {
        let Some(request) = self.request_mut() else {
            bail!("not asking to join");
        };
        match request.confirm(check_id, matches) {
            ConfirmOutcome::Confirmed => {
                self.enter_group();
                Ok(())
            }
            ConfirmOutcome::Mismatch => {
                self.lose(LostReason::Mismatch);
                Ok(())
            }
            ConfirmOutcome::Stale => bail!("that check is not this request's"),
        }
    }

    fn on_check(&mut self, from: SocketId, check: CheckFields) {
        let now = self.clock.now();
        let check_id = check.check_id;
        let member = check.member.clone();
        let Some(request) = self.request_mut() else {
            return;
        };
        match request.on_check(from, check, now) {
            Ok((reveal, code)) => {
                self.send_direct(from, reveal);
                self.emit(ProtectedGroupEvent::Code {
                    role: CodeRole::Joiner,
                    check_id,
                    joiner: self.own_socket.unwrap_or_default(),
                    peer_name: member,
                    code: code.display(),
                });
            }
            Err(busy) => self.send_direct(from, busy),
        }
    }

    /// Holds the Welcome until the joiner has said the codes match. Only one from the socket and
    /// check this request revealed to is kept.
    fn on_welcome(&mut self, from: SocketId, check_id: [u8; 16], welcome: Vec<u8>) {
        let Some(joining) = self.joining.as_mut() else {
            return;
        };
        let JoinPhase::Asking { request, held, .. } = &mut joining.phase else {
            return;
        };
        if request.checker() != Some((from, check_id)) || held.is_some() {
            return;
        }
        *held = Some(HeldWelcome {
            from,
            check_id,
            welcome,
        });
        if request.is_confirmed() {
            self.enter_group();
        }
    }

    /// Enters the group from a held Welcome, after the joiner confirmed, and only if the member
    /// that checked signed it for the group it named.
    fn enter_group(&mut self) {
        let Some(joining) = self.joining.as_mut() else {
            return;
        };
        let phase = std::mem::replace(&mut joining.phase, JoinPhase::Waiting);
        let (request, pending, held) = match phase {
            JoinPhase::Asking {
                request,
                pending,
                held: Some(held),
            } => (request, pending, held),
            other => {
                joining.phase = other;
                return;
            }
        };
        let staged = match pending.stage(&held.welcome) {
            Ok(staged) => staged,
            Err(e) => {
                curia::warn!("protected group: an unreadable Welcome", {
                    channel_id: self.group_id.clone(),
                    error: e.to_string(),
                });
                self.lose(LostReason::ProtocolFailure);
                return;
            }
        };
        let Some(sender_key) = staged.sender_signature_key() else {
            self.lose(LostReason::ProtocolFailure);
            return;
        };
        if let Err(refusal) = request.accept_welcome(
            held.check_id,
            held.from,
            &sender_key,
            &staged.mls_group_id(),
        ) {
            curia::warn!("protected group: refused a Welcome", {
                channel_id: self.group_id.clone(),
                defect: format!("{refusal:?}"),
            });
            self.lose(LostReason::ProtocolFailure);
            return;
        }
        match pending.finish(staged) {
            Ok(state) => self.become_member(state),
            Err(e) => {
                curia::warn!("protected group: a checked Welcome did not join", {
                    channel_id: self.group_id.clone(),
                    error: e.to_string(),
                });
                self.lose(LostReason::ProtocolFailure);
            }
        }
    }

    fn request(&self) -> Option<&JoinerRequest> {
        match self.joining.as_ref().map(|j| &j.phase) {
            Some(JoinPhase::Asking { request, .. }) => Some(request),
            _ => None,
        }
    }

    fn request_mut(&mut self) -> Option<&mut JoinerRequest> {
        match self.joining.as_mut().map(|j| &mut j.phase) {
            Some(JoinPhase::Asking { request, .. }) => Some(request),
            _ => None,
        }
    }

    /// Whether the member on `socket` is the one checking this joiner.
    pub(super) fn is_checker(&self, socket: SocketId) -> bool {
        self.request()
            .and_then(JoinerRequest::checker)
            .is_some_and(|(checker, _)| checker == socket)
    }

    pub(super) fn holds_welcome(&self) -> bool {
        matches!(
            self.joining.as_ref().map(|j| &j.phase),
            Some(JoinPhase::Asking { held: Some(_), .. })
        )
    }

    fn become_member(&mut self, state: MlsGroupState) {
        let buffered = self
            .joining
            .take()
            .map(|joining| joining.buffered_commits)
            .unwrap_or_default();
        let joined_at = state.epoch();
        self.state = Some(state);

        // Commits accepted while the Welcome was in flight: those built on the joined epoch or
        // later still apply, earlier ones are already inside the Welcome.
        for (epoch, body) in buffered {
            if epoch >= joined_at {
                self.on_commit(epoch, &body);
                if self.lost {
                    return;
                }
            }
        }
        self.after_join();
    }

    /// Installs keys and tells the app this client holds the group.
    pub(super) fn after_join(&mut self) {
        self.install_epoch_keys();
        self.last_epoch_change = self.clock.now();
        let Some(state) = self.state.as_ref() else {
            return;
        };
        self.emit(ProtectedGroupEvent::Joined {
            epoch: state.epoch(),
            safety_code: state.safety_code().digits(),
        });
        self.emit(ProtectedGroupEvent::RosterChanged(self.roster()));
        if self.own_socket.is_some() {
            self.announce_binding();
        }
    }
}
