use std::collections::{HashMap, VecDeque};

use bvc_client_lib::groups::protected::MlsGroupState;
use bytes::Bytes;
use common::structs::group_key::{
    AdmissionMessage, GroupKeyFrame, GroupKeyPayload, GroupKeyRecipient, SocketId,
};

use super::Hostility;

/// The server's relay rules, in process: every commit to every socket in order, `Any` never routes
/// back to the sender, `Members` reaches every announced socket but the sender as `Asked` with the
/// sender's identity, a closed socket is announced as `Gone`. A [`Hostility`] layers an attack on
/// top.
pub struct TestRelay {
    sockets: Vec<SocketId>,
    identities: HashMap<SocketId, String>,
    inboxes: HashMap<SocketId, VecDeque<GroupKeyFrame>>,
    announced: Vec<SocketId>,
    next_socket: SocketId,
    cursor: usize,
    pub hostility: Hostility,
    first_commit: Option<GroupKeyFrame>,
}

impl TestRelay {
    pub const IMPOSTOR: SocketId = 999;

    pub fn new() -> Self {
        Self {
            sockets: Vec::new(),
            identities: HashMap::new(),
            inboxes: HashMap::new(),
            announced: Vec::new(),
            next_socket: 1,
            cursor: 0,
            hostility: Hostility::None,
            first_commit: None,
        }
    }

    /// Opens a socket for `identity`, the canonical identity the server authenticated.
    pub fn connect(&mut self, identity: &str) -> SocketId {
        let id = self.next_socket;
        self.next_socket += 1;
        self.sockets.push(id);
        self.identities.insert(id, identity.to_string());
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
        let body = match (self.hostility, payload) {
            (
                Hostility::AbortReveals,
                Some(GroupKeyPayload::Admission(AdmissionMessage::Reveal { .. })),
            ) => return,
            (
                Hostility::SwapKeyPackage,
                Some(GroupKeyPayload::Admission(AdmissionMessage::Reveal {
                    check_id, nonce, ..
                })),
            ) => {
                let (_pending, forged) =
                    MlsGroupState::key_package(&"minecraft:Server".parse().unwrap()).unwrap();
                GroupKeyPayload::Admission(AdmissionMessage::Reveal {
                    check_id,
                    key_package: Bytes::from(forged),
                    nonce,
                })
                .to_bytes()
                .unwrap()
            }
            (
                Hostility::SwapMemberNonce,
                Some(GroupKeyPayload::Admission(AdmissionMessage::Check {
                    check_id,
                    signature_key,
                    mls_group_id,
                    member,
                    ..
                })),
            ) => GroupKeyPayload::Admission(AdmissionMessage::Check {
                check_id,
                nonce: [0x5E; 32],
                signature_key,
                mls_group_id,
                member,
            })
            .to_bytes()
            .unwrap(),
            _ => body,
        };
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
            GroupKeyRecipient::Members => {
                let identity = self.identities.get(&from).cloned().unwrap_or_default();
                let members: Vec<SocketId> =
                    self.announced.iter().copied().filter(|s| *s != from).collect();
                for member in members {
                    self.deliver(
                        member,
                        GroupKeyFrame::Asked {
                            from,
                            identity: identity.clone(),
                            body: body.clone(),
                        },
                    );
                }
                return;
            }
        };
        self.deliver(target, GroupKeyFrame::Delivered { from, body });
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
