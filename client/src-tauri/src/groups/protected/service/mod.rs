mod admitting;
mod clock;
mod committer_rank;
mod event;
mod join_phase;
mod joining;
mod joining_state;
mod lost_reason;
mod membership;
mod pending_commit;
mod pending_kind;
mod socket_bindings;
mod system_clock;

pub use clock::Clock;
pub use committer_rank::CommitterRank;
pub use event::ProtectedGroupEvent;
pub use lost_reason::LostReason;
pub use system_clock::SystemClock;

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use bytes::Bytes;
use common::structs::group_key::{GroupKeyFrame, GroupKeyPayload, SocketId};
use tokio::sync::mpsc;

use super::{AdmissionRateLimiter, AdmitterSession, GroupPassword, MlsGroupState, SealGate};
use join_phase::JoinPhase;
use joining_state::JoiningState;
use pending_commit::PendingCommit;
use pending_kind::PendingKind;
use socket_bindings::SocketBindings;

/// One client's membership of one protected group: admission, MLS commits, and the timers that
/// ratchet the group, driven by frames from the key socket.
///
/// Synchronous on purpose. A caller owns the socket and the clock tick and hands frames in;
/// everything this produces goes out through bounded queues. That keeps every protocol decision
/// testable without a runtime, a socket or a server.
pub struct GroupKeyService<C: Clock> {
    group_id: String,
    identity: common::PlayerIdentity,
    password: GroupPassword,
    clock: C,
    outbound: mpsc::Sender<GroupKeyFrame>,
    events: mpsc::Sender<ProtectedGroupEvent>,
    gate: Arc<SealGate>,
    // This service's session on the gate; installs and clears from any other session are ignored.
    gate_session: u64,
    own_socket: Option<SocketId>,
    state: Option<MlsGroupState>,
    joining: Option<JoiningState>,
    pending: Option<PendingCommit>,
    admissions: HashMap<SocketId, AdmitterSession>,
    queued_adds: Vec<(SocketId, Vec<u8>)>,
    bindings: SocketBindings,
    removals: Vec<(u32, Instant)>,
    last_epoch_change: Instant,
    limiter: AdmissionRateLimiter,
    lost: bool,
}

impl<C: Clock> GroupKeyService<C> {
    const MAX_BUFFERED_COMMITS: usize = 256;
    const MAX_CONCURRENT_ADMISSIONS: usize = 16;

    /// Creates the group at epoch 0 with this client as its only member.
    pub fn create(
        group_id: String,
        identity: common::PlayerIdentity,
        password: GroupPassword,
        clock: C,
        outbound: mpsc::Sender<GroupKeyFrame>,
        events: mpsc::Sender<ProtectedGroupEvent>,
        gate: Arc<SealGate>,
    ) -> anyhow::Result<Self> {
        let state = MlsGroupState::create(&identity)?;
        let mut service = Self::new(group_id, identity, password, clock, outbound, events, gate);
        service.state = Some(state);
        service.after_join();
        Ok(service)
    }

    /// Joins through admission, which starts when the key socket reports its id.
    pub fn join(
        group_id: String,
        identity: common::PlayerIdentity,
        password: GroupPassword,
        clock: C,
        outbound: mpsc::Sender<GroupKeyFrame>,
        events: mpsc::Sender<ProtectedGroupEvent>,
        gate: Arc<SealGate>,
    ) -> anyhow::Result<Self> {
        let mut service = Self::new(group_id, identity, password, clock, outbound, events, gate);
        service.joining = Some(JoiningState {
            phase: JoinPhase::Waiting,
            attempts: 0,
            buffered_commits: Vec::new(),
        });
        Ok(service)
    }

    fn new(
        group_id: String,
        identity: common::PlayerIdentity,
        password: GroupPassword,
        clock: C,
        outbound: mpsc::Sender<GroupKeyFrame>,
        events: mpsc::Sender<ProtectedGroupEvent>,
        gate: Arc<SealGate>,
    ) -> Self {
        gate.set_protected(true);
        let gate_session = gate.begin_session();
        let now = clock.now();
        Self {
            group_id,
            identity,
            password,
            clock,
            outbound,
            events,
            gate,
            gate_session,
            own_socket: None,
            state: None,
            joining: None,
            pending: None,
            admissions: HashMap::new(),
            queued_adds: Vec::new(),
            bindings: SocketBindings::default(),
            removals: Vec::new(),
            last_epoch_change: now,
            limiter: AdmissionRateLimiter::new(),
            lost: false,
        }
    }

    pub fn on_frame(&mut self, frame: GroupKeyFrame) {
        if self.lost {
            return;
        }
        match frame {
            GroupKeyFrame::Welcome { socket } => {
                self.own_socket = Some(socket);
                if self.joining.is_some() {
                    self.start_join_attempt();
                } else {
                    self.announce_binding();
                }
            }
            GroupKeyFrame::Commit { epoch, body } => {
                if let Some(joining) = self.joining.as_mut() {
                    // Anyone can send commits; a joiner that buffered without limit could be made
                    // to hold any amount of them. More than this in one admission is not a group
                    // ratcheting, it is a flood.
                    if joining.buffered_commits.len() >= Self::MAX_BUFFERED_COMMITS {
                        self.lose(LostReason::ProtocolFailure);
                        return;
                    }
                    joining.buffered_commits.push((epoch, body));
                } else {
                    self.on_commit(epoch, &body);
                }
            }
            GroupKeyFrame::Stale { epoch } => self.on_stale(epoch),
            GroupKeyFrame::Gone { member } => self.on_gone(member),
            GroupKeyFrame::Delivered { from, body } => match GroupKeyPayload::from_bytes(&body) {
                Ok(GroupKeyPayload::Admission(message)) => self.on_admission(from, message),
                Ok(GroupKeyPayload::Mls(message)) => self.on_application(from, &message),
                Err(_) => log::debug!("protected group: undecodable delivered frame"),
            },
            // Client-originated frames never arrive from the relay.
            GroupKeyFrame::Broadcast(_) | GroupKeyFrame::Direct { .. } => {}
        }
    }

    /// Drives every timer: admission retries, commit fallbacks, the periodic ratchet.
    pub fn tick(&mut self) {
        if self.lost {
            return;
        }
        if self.joining.is_some() {
            self.tick_joining();
        } else {
            self.tick_member();
        }
    }

    pub fn on_socket_closed(&mut self, reason: Option<&str>) {
        let reason = match reason {
            Some("NoAdmitterOnline") => LostReason::NoAdmitter,
            _ => LostReason::SocketClosed,
        };
        self.lose(reason);
    }

    /// Leaves without a commit: the socket closing is what tells the others, and the lowest
    /// remaining member removes this leaf.
    pub fn leave(&mut self) {
        self.lost = true;
        self.wipe();
    }

    pub fn epoch(&self) -> Option<u64> {
        self.state.as_ref().map(MlsGroupState::epoch)
    }

    pub fn roster(&self) -> Vec<String> {
        self.state
            .as_ref()
            .map(|state| state.roster().iter().map(|m| m.identity.to_string()).collect())
            .unwrap_or_default()
    }

    pub fn safety_code(&self) -> Option<String> {
        self.state.as_ref().map(|state| state.safety_code().digits())
    }

    fn lose(&mut self, reason: LostReason) {
        if self.lost {
            return;
        }
        self.lost = true;
        self.wipe();
        self.emit(ProtectedGroupEvent::Lost(reason));
    }

    fn wipe(&mut self) {
        self.gate.clear_session(self.gate_session);
        self.state = None;
        self.joining = None;
        self.pending = None;
        self.admissions.clear();
        self.queued_adds.clear();
        self.bindings.clear();
        self.removals.clear();
    }

    fn send(&self, frame: GroupKeyFrame) {
        if self.outbound.try_send(frame).is_err() {
            log::warn!("protected group: key socket queue full, frame dropped");
        }
    }

    fn send_payload(&self, frame: impl FnOnce(Bytes) -> GroupKeyFrame, payload: GroupKeyPayload) {
        match payload.to_bytes() {
            Ok(body) => self.send(frame(body)),
            Err(e) => log::error!("protected group: could not encode a payload: {e}"),
        }
    }

    fn emit(&self, event: ProtectedGroupEvent) {
        let _ = self.events.try_send(event);
    }
}
