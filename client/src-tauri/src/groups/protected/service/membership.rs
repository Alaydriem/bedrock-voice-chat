use std::time::{Duration, Instant};

use common::structs::group_key::{GroupKeyFrame, GroupKeyPayload, SocketId};
use tauri_plugin_curia::curia;

use super::{
    Clock, CommitterRank, GroupKeyService, JoinPhase, LostReason, PendingCommit, PendingKind,
    ProtectedGroupEvent,
};

impl<C: Clock> GroupKeyService<C> {
    const PERIODIC: Duration = Duration::from_secs(5 * 60);
    const BINDING: &'static [u8] = b"bvc:bind";

    /// The relay delivers every commit, from anyone, in one order. The first that is valid for this
    /// member's epoch wins; anything else — a stranger's garbage, a commit that lost the race, a
    /// replay — is ignored rather than allowed to end the group.
    pub(super) fn on_commit(&mut self, epoch: u64, body: &[u8]) {
        let Some(current) = self.state.as_ref().map(|s| s.epoch()) else {
            return;
        };
        let before = self.roster();

        if epoch == current && self.pending.as_ref().is_some_and(|p| p.bytes == body) {
            let pending = self.pending.take();
            let leaves_before = self.leaves();
            if let Some(Err(e)) = self.state.as_mut().map(|s| s.merge_pending()) {
                curia::warn!("protected group: could not merge an accepted commit", {
                    channel_id: self.group_id.clone(),
                    error: e.to_string(),
                });
                self.lose(LostReason::ProtocolFailure);
                return;
            }
            self.after_epoch_change(before);
            if let Some(PendingCommit {
                kind: PendingKind::Add {
                    joiner, welcome, ..
                },
                ..
            }) = pending
            {
                // The admitter alone knows the joiner's socket before the joiner announces. Bound
                // now, a joiner that drops before announcing is still removed when it goes,
                // rather than left in the group as a leaf nobody holds the key for.
                if let Some(leaf) = self.leaves().into_iter().find(|l| !leaves_before.contains(l)) {
                    self.bindings.bind_unannounced(joiner, leaf);
                }
                self.deliver_welcome(joiner, &welcome);
            }
            self.try_commit_next_add();
            return;
        }

        // Built on another epoch: a replay, a commit that lost an earlier race, or a stranger's
        // label. None of them can apply here.
        if epoch != current {
            return;
        }

        let Some(state) = self.state.as_mut() else {
            return;
        };
        if let Err(e) = state.process(body) {
            curia::debug!("protected group: ignored a commit that is not valid for this epoch", {
                channel_id: self.group_id.clone(),
                error: e.to_string(),
            });
            return;
        }
        // Another member's commit won this epoch, so ours, built on the same epoch, lost.
        self.requeue_pending();
        if self.state.as_ref().is_some_and(|s| !s.is_active()) {
            self.lose(LostReason::SocketClosed);
            return;
        }
        self.after_epoch_change(before);
        self.try_commit_next_add();
    }

    /// The relay refused a commit; `epoch` is the one the next commit must build on.
    pub(super) fn on_stale(&mut self, epoch: u64) {
        if self.pending.as_ref().is_some_and(|p| p.epoch < epoch) {
            self.requeue_pending();
        }
    }

    pub(super) fn on_gone(&mut self, socket: SocketId) {
        if let Some(joining) = self.joining.as_ref() {
            if matches!(&joining.phase, JoinPhase::AwaitingWelcome { admitter, .. } if *admitter == socket)
            {
                self.start_join_attempt();
            }
            return;
        }
        self.admissions.remove(&socket);
        self.queued_adds.retain(|(joiner, _)| *joiner != socket);
        if let Some(leaf) = self.bindings.leaf_of(socket) {
            self.bindings.forget(socket);
            self.removals.push((leaf, self.clock.now()));
            self.process_removals(self.clock.now());
        }
    }

    pub(super) fn on_application(&mut self, from: SocketId, message: &[u8]) {
        let Some(state) = self.state.as_mut() else {
            return;
        };
        match state.open_application(message) {
            Ok((leaf, data)) if data == Self::BINDING => {
                // A binding this member has not seen means a member it has not heard from — a
                // newcomer, most often, who joined after everyone else announced. Answering with
                // our own is how the newcomer learns ours; one known already gets no answer, so
                // the exchange ends.
                if self.bindings.bind_announced(from, leaf) {
                    self.announce_binding();
                }
            }
            Ok(_) => {}
            Err(e) => curia::debug!("protected group: unreadable application message", {
                channel_id: self.group_id.clone(),
                error: e.to_string(),
            }),
        }
    }

    /// Tells the other members which socket is this member's leaf, authenticated by the leaf's
    /// own key, so they can remove it when the socket closes.
    pub(super) fn announce_binding(&mut self) {
        let Some(state) = self.state.as_mut() else {
            return;
        };
        match state.seal_application(Self::BINDING) {
            Ok(message) => self.send_payload(
                GroupKeyFrame::Broadcast,
                GroupKeyPayload::Mls(message.into()),
            ),
            Err(e) => curia::warn!("protected group: could not announce this socket", {
                channel_id: self.group_id.clone(),
                error: e.to_string(),
            }),
        }
    }

    pub(super) fn install_epoch_keys(&mut self) {
        let Some(state) = self.state.as_ref() else {
            return;
        };
        self.gate.install_epoch(
            self.gate_session,
            state.own_leaf(),
            state.epoch(),
            state.sframe_base_keys(),
            &state.roster(),
            self.clock.now(),
        );
    }

    /// No pending-commit timeout: the relay delivers every commit back over an ordered socket, so a
    /// commit either comes back or another valid one for the epoch arrives first. Timing one out
    /// would make its late echo unrecognisable.
    pub(super) fn tick_member(&mut self) {
        let now = self.clock.now();
        self.process_removals(now);
        self.periodic_ratchet(now);
        self.try_commit_next_add();
    }

    fn after_epoch_change(&mut self, before: Vec<String>) {
        self.install_epoch_keys();
        self.last_epoch_change = self.clock.now();
        let Some(state) = self.state.as_ref() else {
            return;
        };
        let leaves: Vec<u32> = state.roster().iter().map(|m| m.leaf).collect();
        self.removals.retain(|(leaf, _)| leaves.contains(leaf));
        self.emit(ProtectedGroupEvent::EpochChanged {
            epoch: state.epoch(),
            safety_code: state.safety_code().digits(),
        });
        let after = self.roster();
        if after != before {
            self.emit(ProtectedGroupEvent::RosterChanged(after));
        }
    }

    /// Drops a commit that lost or went unanswered; a lost Add goes back to the front of the
    /// queue to be rebuilt on the next epoch.
    fn requeue_pending(&mut self) {
        let Some(pending) = self.pending.take() else {
            return;
        };
        if let Some(state) = self.state.as_mut() {
            state.discard_pending();
        }
        if let PendingKind::Add {
            joiner,
            key_package,
            ..
        } = pending.kind
        {
            self.queued_adds.insert(0, (joiner, key_package));
        }
    }

    fn process_removals(&mut self, now: Instant) {
        if self.pending.is_some() {
            return;
        }
        let Some(state) = self.state.as_ref() else {
            return;
        };
        let leaves: Vec<u32> = state.roster().iter().map(|m| m.leaf).collect();
        let own = state.own_leaf();
        self.removals.retain(|(leaf, _)| leaves.contains(leaf));
        let Some(&(leaf, since)) = self.removals.first() else {
            return;
        };
        let Some(delay) = CommitterRank::removal_delay(&leaves, leaf, own) else {
            return;
        };
        if now.duration_since(since) < delay {
            return;
        }
        let Some(state) = self.state.as_mut() else {
            return;
        };
        let epoch = state.epoch();
        match state.remove(leaf) {
            Ok(commit) => self.send_commit(PendingKind::Remove, commit, epoch),
            Err(e) => curia::warn!("protected group: could not remove a leaf", {
                channel_id: self.group_id.clone(),
                leaf: leaf,
                error: e.to_string(),
            }),
        }
    }

    fn periodic_ratchet(&mut self, now: Instant) {
        if self.pending.is_some() || !self.removals.is_empty() {
            return;
        }
        let Some(state) = self.state.as_ref() else {
            return;
        };
        let leaves: Vec<u32> = state.roster().iter().map(|m| m.leaf).collect();
        let Some(delay) = CommitterRank::periodic_delay(&leaves, state.epoch(), state.own_leaf())
        else {
            return;
        };
        if now.duration_since(self.last_epoch_change) < Self::PERIODIC + delay {
            return;
        }
        let Some(state) = self.state.as_mut() else {
            return;
        };
        let epoch = state.epoch();
        match state.self_update() {
            Ok(commit) => self.send_commit(PendingKind::Update, commit, epoch),
            Err(e) => curia::warn!("protected group: could not commit the periodic ratchet", {
                channel_id: self.group_id.clone(),
                error: e.to_string(),
            }),
        }
    }

    fn leaves(&self) -> Vec<u32> {
        self.state
            .as_ref()
            .map(|state| state.roster().iter().map(|m| m.leaf).collect())
            .unwrap_or_default()
    }

    fn send_commit(&mut self, kind: PendingKind, commit: Vec<u8>, epoch: u64) {
        self.pending = Some(PendingCommit {
            kind,
            bytes: commit.clone(),
            epoch,
        });
        self.send(GroupKeyFrame::Commit {
            epoch,
            body: commit.into(),
        });
    }
}
