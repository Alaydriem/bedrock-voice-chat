use std::sync::Arc;
use std::time::Duration;

use bvc_client_lib::groups::protected::{GroupKeyService, GroupPassword, SealGate};
use tokio::sync::mpsc;

use super::{Hostility, TestClock, TestMember, TestRelay};

/// Drives members and a relay against one shared clock until no frame is left in flight.
pub struct TestGroup {
    pub relay: TestRelay,
    pub members: Vec<TestMember>,
    pub clock: TestClock,
}

impl TestGroup {
    pub const ID: &'static str = "ABCD2345";
    pub const PASSWORD: &'static str = "correct horse";

    pub fn new() -> Self {
        Self {
            relay: TestRelay::new(),
            members: Vec::new(),
            clock: TestClock::new(),
        }
    }

    pub fn with_hostility(hostility: Hostility) -> Self {
        let mut group = Self::new();
        group.relay.hostility = hostility;
        group
    }

    pub fn create(&mut self, name: &str) -> usize {
        self.add_member(name, Self::PASSWORD, true)
    }

    pub fn join(&mut self, name: &str) -> usize {
        self.add_member(name, Self::PASSWORD, false)
    }

    pub fn join_with(&mut self, name: &str, password: &str) -> usize {
        self.add_member(name, password, false)
    }

    /// Adds a joiner without letting anything settle, so several joins can race.
    pub fn join_quietly(&mut self, name: &str) -> usize {
        self.add_member_unsettled(name, Self::PASSWORD, false)
    }

    pub fn freeze(&mut self, index: usize) {
        self.members[index].frozen = true;
    }

    fn add_member(&mut self, name: &str, password: &str, create: bool) -> usize {
        let index = self.add_member_unsettled(name, password, create);
        self.pump();
        index
    }

    fn add_member_unsettled(&mut self, name: &str, password: &str, create: bool) -> usize {
        let (outbound_tx, outbound) = mpsc::channel(256);
        let (event_tx, events) = mpsc::channel(256);
        let gate = Arc::new(SealGate::new());
        let identity: common::PlayerIdentity = format!("minecraft:{name}").parse().unwrap();
        let password = GroupPassword::new(password).unwrap();
        let service = if create {
            GroupKeyService::create(
                Self::ID.to_string(),
                identity,
                password,
                self.clock.clone(),
                outbound_tx,
                event_tx,
                gate.clone(),
            )
        } else {
            GroupKeyService::join(
                Self::ID.to_string(),
                identity,
                password,
                self.clock.clone(),
                outbound_tx,
                event_tx,
                gate.clone(),
            )
        }
        .unwrap();
        let socket = self.relay.connect();
        self.members
            .push(TestMember::new(service, socket, gate, outbound, events));
        self.members.len() - 1
    }

    /// Moves frames between members and the relay until nothing moves.
    pub fn pump(&mut self) {
        for _ in 0..10_000 {
            let mut moved = false;
            for member in &mut self.members {
                if !member.active() {
                    continue;
                }
                for frame in member.drain_outbound() {
                    self.relay.handle(member.socket, frame);
                    moved = true;
                }
            }
            for member in &mut self.members {
                if !member.active() {
                    continue;
                }
                for frame in self.relay.take_inbox(member.socket) {
                    member.service.on_frame(frame);
                    moved = true;
                }
                member.collect_events();
            }
            if !moved {
                return;
            }
        }
        panic!("frames never settled");
    }

    /// Advances time in one-second ticks, letting timers fire and settle at each.
    pub fn advance(&mut self, by: Duration) {
        let mut remaining = by;
        while !remaining.is_zero() {
            let step = remaining.min(Duration::from_secs(1));
            self.clock.advance(step);
            remaining -= step;
            for member in &mut self.members {
                if member.active() {
                    member.service.tick();
                }
            }
            self.pump();
        }
    }

    /// The socket drops, as when an app is killed: the relay announces `Gone`.
    pub fn disconnect(&mut self, index: usize) {
        let member = &mut self.members[index];
        member.connected = false;
        member.service.on_socket_closed(None);
        member.collect_events();
        let socket = member.socket;
        self.relay.close(socket);
        self.pump();
    }

    pub fn epoch(&self, index: usize) -> Option<u64> {
        self.members[index].service.epoch()
    }

    pub fn roster(&self, index: usize) -> Vec<String> {
        self.members[index].service.roster()
    }

    pub fn safety_code(&self, index: usize) -> Option<String> {
        self.members[index].service.safety_code()
    }
}
