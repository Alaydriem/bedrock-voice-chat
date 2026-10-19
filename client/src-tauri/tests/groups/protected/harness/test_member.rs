use std::sync::Arc;

use bvc_client_lib::groups::protected::{GroupKeyService, ProtectedGroupEvent, SealGate};
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
}
