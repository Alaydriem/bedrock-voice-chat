use std::sync::Arc;
use std::time::Duration;

use bvc_client_lib::groups::protected::{CodeRole, GroupKeyService, SealGate};
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
        let index = self.add_member_unsettled(name, true);
        self.pump();
        index
    }

    /// Asks to join, and has the first member that can answer admit it, both people pressing
    /// their buttons after comparing equal codes.
    pub fn join(&mut self, name: &str) -> usize {
        let index = self.join_quietly(name);
        self.pump();
        let admitter = (0..self.members.len())
            .find(|m| *m != index && self.members[*m].active() && self.members[*m].joined());
        if let Some(admitter) = admitter {
            self.admit(admitter, index);
        }
        index
    }

    /// Asks to join without letting anything settle or anyone answer, so several joins can race.
    pub fn join_quietly(&mut self, name: &str) -> usize {
        self.add_member_unsettled(name, false)
    }

    /// `member` checks `joiner`, both see the same code, the member admits and the joiner
    /// confirms.
    pub fn admit(&mut self, member: usize, joiner: usize) {
        self.check(member, joiner);
        let (member_check, member_code) = self
            .member_code(member, joiner)
            .expect("the member was shown a code");
        let (joiner_check, joiner_code) = self
            .joiner_code(joiner)
            .expect("the joiner was shown a code");
        assert_eq!(member_code, joiner_code, "the two codes differ");
        self.decide(member, joiner, member_check, true);
        self.confirm(joiner, joiner_check, true);
    }

    pub fn check(&mut self, member: usize, joiner: usize) {
        let socket = self.members[joiner].socket;
        self.members[member]
            .service
            .check(socket)
            .expect("the member can check");
        self.pump();
    }

    pub fn decide(&mut self, member: usize, joiner: usize, check_id: [u8; 16], allow: bool) {
        let socket = self.members[joiner].socket;
        self.members[member]
            .service
            .decide(socket, check_id, allow)
            .expect("the decision applies");
        self.pump();
    }

    pub fn confirm(&mut self, joiner: usize, check_id: [u8; 16], matches: bool) {
        self.members[joiner]
            .service
            .confirm(check_id, matches)
            .expect("the confirmation applies");
        self.pump();
    }

    pub fn member_code(&self, member: usize, joiner: usize) -> Option<([u8; 16], String)> {
        let socket = self.members[joiner].socket;
        self.members[member].code(CodeRole::Member, Some(socket))
    }

    pub fn joiner_code(&self, joiner: usize) -> Option<([u8; 16], String)> {
        self.members[joiner].code(CodeRole::Joiner, None)
    }

    pub fn freeze(&mut self, index: usize) {
        self.members[index].frozen = true;
    }

    fn add_member_unsettled(&mut self, name: &str, create: bool) -> usize {
        let (outbound_tx, outbound) = mpsc::channel(256);
        let (event_tx, events) = mpsc::channel(256);
        let gate = Arc::new(SealGate::new());
        let canonical = format!("minecraft:{name}");
        let identity: common::PlayerIdentity = canonical.parse().unwrap();
        let service = if create {
            GroupKeyService::create(
                Self::ID.to_string(),
                identity,
                self.clock.clone(),
                outbound_tx,
                event_tx,
                gate.clone(),
            )
        } else {
            GroupKeyService::join(
                Self::ID.to_string(),
                identity,
                self.clock.clone(),
                outbound_tx,
                event_tx,
                gate.clone(),
            )
        }
        .unwrap();
        let socket = self.relay.connect(&canonical);
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
