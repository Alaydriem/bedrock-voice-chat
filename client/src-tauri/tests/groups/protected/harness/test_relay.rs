use std::collections::{HashMap, VecDeque};

use bvc_client_lib::groups::protected::{AdmitterSession, GroupPassword, MlsGroupState};
use bytes::Bytes;
use common::structs::group_key::{
    AdmissionMessage, GroupKeyFrame, GroupKeyPayload, GroupKeyRecipient, SocketId,
};

use super::Hostility;

/// The server's relay rules, in process: first commit per epoch wins, `Any` never routes back to
/// the sender, a closed socket is announced as `Gone`. A [`Hostility`] layers an attack on top.
pub struct TestRelay {
    sockets: Vec<SocketId>,
    inboxes: HashMap<SocketId, VecDeque<GroupKeyFrame>>,
    announced: Vec<SocketId>,
    next_socket: SocketId,
    cursor: usize,
    pub hostility: Hostility,
    first_commit: Option<GroupKeyFrame>,
    impostor: Option<AdmitterSession>,
    pub impostor_verified_an_offer: Option<bool>,
}

impl TestRelay {
    pub const IMPOSTOR: SocketId = 999;
    const GROUP: &'static str = "ABCD2345";

    pub fn new() -> Self {
        Self {
            sockets: Vec::new(),
            inboxes: HashMap::new(),
            announced: Vec::new(),
            next_socket: 1,
            cursor: 0,
            hostility: Hostility::None,
            first_commit: None,
            impostor: None,
            impostor_verified_an_offer: None,
        }
    }

    pub fn connect(&mut self) -> SocketId {
        let id = self.next_socket;
        self.next_socket += 1;
        self.sockets.push(id);
        self.inboxes
            .entry(id)
            .or_default()
            .push_back(GroupKeyFrame::Welcome { socket: id });
        id
    }

    /// Delivers a frame the relay made up, as a hostile server or a stranger's socket could.
    pub fn inject(&mut self, to: SocketId, frame: GroupKeyFrame) {
        self.deliver(to, frame);
    }

    /// Sends a frame to every socket, as a commit from a stranger's socket would arrive.
    pub fn inject_all(&mut self, frame: GroupKeyFrame) {
        self.deliver_all(frame, None);
    }

    pub fn close(&mut self, socket: SocketId) {
        self.sockets.retain(|s| *s != socket);
        self.announced.retain(|s| *s != socket);
        self.inboxes.remove(&socket);
        self.deliver_all(GroupKeyFrame::Gone { member: socket }, None);
    }

    pub fn take_inbox(&mut self, socket: SocketId) -> Vec<GroupKeyFrame> {
        self.inboxes
            .get_mut(&socket)
            .map(|inbox| inbox.drain(..).collect())
            .unwrap_or_default()
    }

    pub fn handle(&mut self, from: SocketId, frame: GroupKeyFrame) {
        match frame {
            GroupKeyFrame::Commit { epoch, body } => self.commit(from, epoch, body),
            GroupKeyFrame::Broadcast(body) => {
                if !self.announced.contains(&from) {
                    self.announced.push(from);
                }
                self.deliver_all(GroupKeyFrame::Delivered { from, body }, Some(from))
            }
            GroupKeyFrame::Direct { to, body } => self.direct(from, to, body),
            _ => {}
        }
    }

    fn commit(&mut self, from: SocketId, epoch: u64, body: Bytes) {
        let frame = GroupKeyFrame::Commit { epoch, body };
        if self.hostility == Hostility::ForkOnNextCommit {
            self.deliver(from, frame);
            return;
        }
        // As the server does: every commit to every socket, in order; members pick the winner.
        self.deliver_all(frame.clone(), None);
        if self.hostility == Hostility::ReplayFirstCommit {
            match &self.first_commit {
                None => self.first_commit = Some(frame),
                Some(first) => self.deliver_all(first.clone(), None),
            }
        }
    }

    fn direct(&mut self, from: SocketId, to: GroupKeyRecipient, body: Bytes) {
        let payload = GroupKeyPayload::from_bytes(&body).ok();
        match (self.hostility, payload) {
            (Hostility::PakeToSelf, Some(GroupKeyPayload::Admission(message))) => {
                self.impersonate_admitter(from, message);
                return;
            }
            (
                Hostility::SwapKeyPackage,
                Some(GroupKeyPayload::Admission(AdmissionMessage::KeyPackageOffer { mac, .. })),
            ) => {
                let (_pending, forged) =
                    MlsGroupState::key_package(&"minecraft:Server".parse().unwrap()).unwrap();
                let swapped = GroupKeyPayload::Admission(AdmissionMessage::KeyPackageOffer {
                    key_package: Bytes::from(forged),
                    mac,
                });
                self.route(from, to, swapped.to_bytes().unwrap());
                return;
            }
            (
                Hostility::InjectWelcome,
                Some(GroupKeyPayload::Admission(AdmissionMessage::KeyPackageOffer {
                    key_package,
                    ..
                })),
            ) => {
                let welcome = Self::own_group_welcome(&key_package);
                let forged = GroupKeyPayload::Admission(AdmissionMessage::WelcomeOffer {
                    welcome: Bytes::from(welcome),
                    mac: [0x42; 32],
                });
                // The server chooses `from`, so the forgery claims to be the real admitter.
                let GroupKeyRecipient::Socket(admitter) = to else {
                    return;
                };
                self.deliver(
                    from,
                    GroupKeyFrame::Delivered {
                        from: admitter,
                        body: forged.to_bytes().unwrap(),
                    },
                );
                return;
            }
            _ => {}
        }
        self.route(from, to, body);
    }

    fn route(&mut self, from: SocketId, to: GroupKeyRecipient, body: Bytes) {
        let target = match to {
            GroupKeyRecipient::Socket(target) => target,
            GroupKeyRecipient::Any => {
                let others: Vec<SocketId> =
                    self.announced.iter().copied().filter(|s| *s != from).collect();
                if others.is_empty() {
                    return;
                }
                let pick = others[self.cursor % others.len()];
                self.cursor += 1;
                pick
            }
        };
        self.deliver(target, GroupKeyFrame::Delivered { from, body });
    }

    /// The relay plays admitter with a guessed password, then hands the joiner a Welcome into a
    /// group it runs itself.
    fn impersonate_admitter(&mut self, joiner: SocketId, message: AdmissionMessage) {
        let guess = GroupPassword::new("a guess!").unwrap();
        let reply = match message {
            AdmissionMessage::PakeInit(_) => {
                let (session, reply) =
                    AdmitterSession::respond(Self::GROUP, &guess, message).unwrap();
                self.impostor = Some(session);
                GroupKeyPayload::Admission(reply)
            }
            offer @ AdmissionMessage::KeyPackageOffer { .. } => {
                let AdmissionMessage::KeyPackageOffer { key_package, .. } = offer.clone() else {
                    unreachable!()
                };
                let session = self.impostor.take().expect("PAKE ran first");
                self.impostor_verified_an_offer = Some(session.verify_offer(offer).is_ok());
                let welcome = Self::own_group_welcome(&key_package);
                GroupKeyPayload::Admission(session.seal_welcome(&welcome))
            }
            _ => return,
        };
        self.deliver(
            joiner,
            GroupKeyFrame::Delivered {
                from: Self::IMPOSTOR,
                body: reply.to_bytes().unwrap(),
            },
        );
    }

    fn own_group_welcome(key_package: &[u8]) -> Vec<u8> {
        let mut own = MlsGroupState::create(&"minecraft:Alice".parse().unwrap()).unwrap();
        let staged = own.add(key_package).unwrap();
        own.merge_pending().unwrap();
        staged.welcome
    }

    fn deliver(&mut self, to: SocketId, frame: GroupKeyFrame) {
        if let Some(inbox) = self.inboxes.get_mut(&to) {
            inbox.push_back(frame);
        }
    }

    fn deliver_all(&mut self, frame: GroupKeyFrame, skip: Option<SocketId>) {
        let targets: Vec<SocketId> =
            self.sockets.iter().copied().filter(|s| Some(*s) != skip).collect();
        for target in targets {
            self.deliver(target, frame.clone());
        }
    }
}
