use std::time::Duration;

use common::errors::AdmissionError;
use common::structs::group_key::{
    AdmissionMessage, GroupKeyFrame, GroupKeyPayload, GroupKeyRecipient, SocketId,
};

use super::{Clock, GroupKeyService, JoinPhase, LostReason, ProtectedGroupEvent};
use crate::groups::protected::{JoinerSession, MlsGroupState};

impl<C: Clock> GroupKeyService<C> {
    const ADMITTER_TIMEOUT: Duration = Duration::from_secs(5);
    // Once a KeyPackage is offered and its MAC verified, the admitter is committing an Add. A new
    // attempt with a fresh KeyPackage then could leave the first Add's leaf in the group with no
    // one holding its key, so the joiner waits for this Welcome instead, and gives up rather than
    // retries if it never comes.
    const WELCOME_TIMEOUT: Duration = Duration::from_secs(30);
    const MAX_ATTEMPTS: u32 = 3;

    /// Starts admission against whichever member the relay picks.
    pub(super) fn start_join_attempt(&mut self) {
        let Some(joining) = self.joining.as_mut() else {
            return;
        };
        if joining.attempts >= Self::MAX_ATTEMPTS {
            self.lose(LostReason::AdmitterUnreachable);
            return;
        }
        joining.attempts += 1;

        let (pending, key_package) = match MlsGroupState::key_package(&self.identity) {
            Ok(material) => material,
            Err(e) => {
                log::error!("protected group: could not build a key package: {e}");
                self.lose(LostReason::ProtocolFailure);
                return;
            }
        };
        let (session, init) = JoinerSession::start(&self.group_id, &self.password);
        let now = self.clock.now();
        if let Some(joining) = self.joining.as_mut() {
            joining.phase = JoinPhase::AwaitingReply {
                session,
                pending,
                key_package,
                sent_at: now,
            };
        }
        self.send_payload(
            |body| GroupKeyFrame::Direct {
                to: GroupKeyRecipient::Any,
                body,
            },
            GroupKeyPayload::Admission(init),
        );
    }

    pub(super) fn tick_joining(&mut self) {
        let now = self.clock.now();
        match self.joining.as_ref().map(|j| &j.phase) {
            Some(JoinPhase::AwaitingReply { sent_at, .. })
                if now.duration_since(*sent_at) >= Self::ADMITTER_TIMEOUT =>
            {
                self.start_join_attempt();
            }
            Some(JoinPhase::AwaitingWelcome { sent_at, .. })
                if now.duration_since(*sent_at) >= Self::WELCOME_TIMEOUT =>
            {
                self.lose(LostReason::AdmitterUnreachable);
            }
            _ => {}
        }
    }

    pub(super) fn on_joiner_admission(&mut self, from: SocketId, message: AdmissionMessage) {
        let Some(joining) = self.joining.as_mut() else {
            return;
        };
        let phase = std::mem::replace(&mut joining.phase, JoinPhase::Waiting);
        match (phase, message) {
            (
                JoinPhase::AwaitingReply {
                    session,
                    pending,
                    key_package,
                    ..
                },
                reply @ AdmissionMessage::PakeReply(_),
            ) => match session.on_reply(reply, &key_package) {
                Ok((awaiting, offer)) => {
                    let now = self.clock.now();
                    if let Some(joining) = self.joining.as_mut() {
                        joining.phase = JoinPhase::AwaitingWelcome {
                            awaiting,
                            pending,
                            admitter: from,
                            sent_at: now,
                        };
                    }
                    self.send_payload(
                        |body| GroupKeyFrame::Direct {
                            to: GroupKeyRecipient::Socket(from),
                            body,
                        },
                        GroupKeyPayload::Admission(offer),
                    );
                }
                // A reply that is not a PAKE message can come from any socket, a stranger's
                // included; it costs this attempt, not the whole join.
                Err(AdmissionError::Protocol(detail)) => {
                    log::debug!("protected group: unusable PAKE reply: {detail}");
                    self.start_join_attempt();
                }
                Err(error) => self.lose_admission(error),
            },
            (
                JoinPhase::AwaitingWelcome {
                    awaiting,
                    pending,
                    admitter,
                    ..
                },
                offer @ (AdmissionMessage::WelcomeOffer { .. } | AdmissionMessage::Refused),
            ) if admitter == from => match awaiting.on_welcome(offer) {
                Ok(welcome) => match pending.join(&welcome) {
                    Ok(state) => self.become_member(state),
                    Err(e) => {
                        log::warn!("protected group: a verified Welcome did not join: {e}");
                        self.lose(LostReason::ProtocolFailure);
                    }
                },
                Err(error) => self.lose_admission(error),
            },
            // Anything else — a refusal from a socket other than the admitter, a message out of
            // order — is not this join's answer. Socket ids are guessable, so honouring a stranger's
            // refusal would let anyone end anyone's join.
            (phase, _) => {
                if let Some(joining) = self.joining.as_mut() {
                    joining.phase = phase;
                }
            }
        }
    }

    fn lose_admission(&mut self, error: AdmissionError) {
        match error {
            AdmissionError::Protocol(detail) => {
                log::warn!("protected group: admission protocol error: {detail}");
                self.lose(LostReason::ProtocolFailure);
            }
            _ => self.lose(LostReason::WrongPassword),
        }
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
