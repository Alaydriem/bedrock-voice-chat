use std::sync::Arc;

use bvc_client_lib::groups::protected::{CodeRole, GroupKeyService, ProtectedGroupEvent, SealGate};
use common::structs::group_key::{GroupKeyFrame, SocketId};
use tokio::sync::mpsc;

use super::TestClock;

/// One member's client: its service, the queues it writes to, and its audio gate.
pub struct TestMember {
    pub service: GroupKeyService<TestClock>,
    pub socket: SocketId,
    pub gate: Arc<SealGate>,
    pub events: Vec<ProtectedGroupEvent>,
    pub connected: bool,
    /// Still connected, but neither reads nor writes: a client that hung.
    pub frozen: bool,
    outbound: mpsc::Receiver<GroupKeyFrame>,
    event_rx: mpsc::Receiver<ProtectedGroupEvent>,
}

impl TestMember {
    pub fn new(
        service: GroupKeyService<TestClock>,
        socket: SocketId,
        gate: Arc<SealGate>,
        outbound: mpsc::Receiver<GroupKeyFrame>,
        event_rx: mpsc::Receiver<ProtectedGroupEvent>,
    ) -> Self {
        Self {
            service,
            socket,
            gate,
            events: Vec::new(),
            connected: true,
            frozen: false,
            outbound,
            event_rx,
        }
    }

    pub fn drain_outbound(&mut self) -> Vec<GroupKeyFrame> {
        let mut frames = Vec::new();
        while let Ok(frame) = self.outbound.try_recv() {
            frames.push(frame);
        }
        frames
    }

    pub fn collect_events(&mut self) {
        while let Ok(event) = self.event_rx.try_recv() {
            self.events.push(event);
        }
    }

    pub fn lost(&self) -> Option<&ProtectedGroupEvent> {
        self.events
            .iter()
            .find(|e| matches!(e, ProtectedGroupEvent::Lost(_)))
    }

    pub fn active(&self) -> bool {
        self.connected && !self.frozen
    }

    pub fn joined(&self) -> bool {
        self.events
            .iter()
            .any(|e| matches!(e, ProtectedGroupEvent::Joined { .. }))
    }

    /// Join request cards raised on this member's screen, in order.
    pub fn requests(&self) -> Vec<(SocketId, String)> {
        self.events
            .iter()
            .filter_map(|event| match event {
                ProtectedGroupEvent::Request { joiner, name, .. } => Some((*joiner, name.clone())),
                _ => None,
            })
            .collect()
    }

    /// The latest code this client was shown in `role`, for `joiner` when a member.
    pub fn code(&self, role: CodeRole, joiner: Option<SocketId>) -> Option<([u8; 16], String)> {
        self.events.iter().rev().find_map(|event| match event {
            ProtectedGroupEvent::Code {
                role: shown,
                check_id,
                joiner: for_joiner,
                code,
                ..
            } if *shown == role && joiner.is_none_or(|j| j == *for_joiner) => {
                Some((*check_id, code.clone()))
            }
            _ => None,
        })
    }

    pub fn has(&self, wanted: impl Fn(&ProtectedGroupEvent) -> bool) -> bool {
        self.events.iter().any(wanted)
    }
}
