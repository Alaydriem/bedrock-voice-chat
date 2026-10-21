mod close_outcome;
mod group_relay_state;
mod rate_verdict;
mod relay_outcome;
mod socket_entry;

pub use close_outcome::CloseOutcome;
pub use relay_outcome::RelayOutcome;

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use common::errors::ProtectedGroupError;
use common::structs::group_key::{GroupKeyFrame, GroupKeyRecipient, SocketId};
use moka::future::Cache;
use tokio::sync::{Mutex, mpsc};

use group_relay_state::GroupRelayState;
use rate_verdict::RateVerdict;
use socket_entry::SocketEntry;

/// Sequences and routes a protected group's key socket frames.
///
/// It never parses a frame body: the server holds no MLS or SFrame code, and the bodies
/// are encrypted, public, or safe to relay. What it owns is order — every socket receives every
/// commit in the same sequence, so members agree on which valid commit came first — and delivery
/// between sockets.
pub struct GroupKeyRelayService {
    groups: Cache<String, Arc<Mutex<GroupRelayState>>>,
    next_socket: AtomicU64,
}

impl GroupKeyRelayService {
    pub const MAX_FRAME_BYTES: usize = 256 * 1024;
    pub const QUEUE_DEPTH: usize = 256;
    pub const CREATOR_DEADLINE: Duration = Duration::from_secs(60);
    // A leak guard only: an entry is removed when its last socket closes, and every frame
    // touches it, so an active group never idles this long.
    const IDLE_EVICTION: Duration = Duration::from_secs(24 * 60 * 60);
    const MAX_GROUPS: u64 = 4096;

    pub fn new() -> Self {
        Self {
            groups: Cache::builder()
                .time_to_idle(Self::IDLE_EVICTION)
                .max_capacity(Self::MAX_GROUPS)
                .build(),
            next_socket: AtomicU64::new(1),
        }
    }

    pub fn new_shared() -> Arc<Self> {
        Arc::new(Self::new())
    }

    /// Opens a socket on `group` and sends it `Welcome` with its id. A second socket for the
    /// same identity displaces the first, which the other members see as `Gone`.
    pub async fn open(
        &self,
        group: &str,
        identity: common::PlayerIdentity,
    ) -> (SocketId, mpsc::Receiver<GroupKeyFrame>) {
        let id = self.next_socket.fetch_add(1, Ordering::Relaxed);
        let (tx, rx) = mpsc::channel(Self::QUEUE_DEPTH);
        let _ = tx.try_send(GroupKeyFrame::Welcome { socket: id });

        let state = self
            .groups
            .get_with(group.to_string(), async { Arc::new(Mutex::new(GroupRelayState::default())) })
            .await;
        let mut state = state.lock().await;
        let displaced: Vec<SocketId> = state
            .sockets
            .iter()
            .filter(|s| s.identity == identity)
            .map(|s| s.id)
            .collect();
        for old in displaced {
            state.remove(old);
        }
        state.sockets.push(SocketEntry::new(id, identity, tx));
        (id, rx)
    }

    pub async fn handle(
        &self,
        group: &str,
        from: SocketId,
        frame: GroupKeyFrame,
        now: Instant,
    ) -> RelayOutcome {
        let Some(state) = self.groups.get(group).await else {
            return RelayOutcome::Dropped(ProtectedGroupError::NotMember);
        };
        let mut state = state.lock().await;

        let Some(socket) = state.find(from) else {
            return RelayOutcome::Dropped(ProtectedGroupError::NotMember);
        };
        let size = Self::body_len(&frame);
        match socket.meter(size, now) {
            RateVerdict::Within => {}
            RateVerdict::Over => return RelayOutcome::Dropped(ProtectedGroupError::RateLimited),
            RateVerdict::Close => {
                state.remove(from);
                return RelayOutcome::Dropped(ProtectedGroupError::RateLimited);
            }
        }
        if size > Self::MAX_FRAME_BYTES {
            return RelayOutcome::Dropped(ProtectedGroupError::FrameTooLarge);
        }

        match frame {
            // Every commit goes to every socket, the sender included, in the one order this lock
            // gives them. Members apply the first that is valid for their epoch: only they can
            // tell a member's commit from a stranger's, so the relay must not pick a winner.
            GroupKeyFrame::Commit { epoch, body } => {
                state.deliver_all(GroupKeyFrame::Commit { epoch, body }, None);
                RelayOutcome::Routed
            }
            GroupKeyFrame::Broadcast(body) => {
                if let Some(socket) = state.find(from) {
                    socket.announced = true;
                }
                state.deliver_all(GroupKeyFrame::Delivered { from, body }, Some(from));
                RelayOutcome::Routed
            }
            GroupKeyFrame::Direct { to, body } => {
                let target = match to {
                    GroupKeyRecipient::Any => match state.pick_other(from) {
                        Some(target) => target,
                        None => return RelayOutcome::NoAdmitter,
                    },
                    GroupKeyRecipient::Socket(target) => target,
                    GroupKeyRecipient::Members => {
                        return Self::deliver_members(&mut state, from, body, now);
                    }
                };
                if target == from || state.find(target).is_none() {
                    return RelayOutcome::Dropped(ProtectedGroupError::NotMember);
                }
                state.deliver(target, GroupKeyFrame::Delivered { from, body });
                RelayOutcome::Routed
            }
            // Only the server originates these; a client that sends one is refused.
            GroupKeyFrame::Stale { .. }
            | GroupKeyFrame::Gone { .. }
            | GroupKeyFrame::Delivered { .. }
            | GroupKeyFrame::Asked { .. }
            | GroupKeyFrame::Welcome { .. } => {
                RelayOutcome::Dropped(ProtectedGroupError::NotMember)
            }
        }
    }

    /// A join request goes to every announced member at once, so whoever is at their screen can
    /// check the joiner. A socket may send one per `SocketEntry::MEMBERS_INTERVAL`.
    fn deliver_members(
        state: &mut GroupRelayState,
        from: SocketId,
        body: bytes::Bytes,
        now: Instant,
    ) -> RelayOutcome {
        let members = state.members_except(from);
        if members.is_empty() {
            return RelayOutcome::NoAdmitter;
        }
        let Some(sender) = state.find(from) else {
            return RelayOutcome::Dropped(ProtectedGroupError::NotMember);
        };
        if !sender.permits_members_frame(now) {
            return RelayOutcome::Dropped(ProtectedGroupError::RateLimited);
        }
        let identity = sender.identity.to_string();
        for member in members {
            state.deliver(
                member,
                GroupKeyFrame::Asked {
                    from,
                    identity: identity.clone(),
                    body: body.clone(),
                },
            );
        }
        RelayOutcome::Routed
    }

    pub async fn close(&self, group: &str, socket: SocketId) -> CloseOutcome {
        let Some(state) = self.groups.get(group).await else {
            return CloseOutcome::GroupEmpty;
        };
        let empty = {
            let mut state = state.lock().await;
            state.remove(socket);
            state.sockets.is_empty()
        };
        if empty {
            self.groups.invalidate(group).await;
            CloseOutcome::GroupEmpty
        } else {
            CloseOutcome::MembersRemain
        }
    }

    pub async fn has_sockets(&self, group: &str) -> bool {
        match self.groups.get(group).await {
            Some(state) => !state.lock().await.sockets.is_empty(),
            None => false,
        }
    }

    /// Runs `on_expire` after [`Self::CREATOR_DEADLINE`] if the group still has no socket: a
    /// creator who never connects leaves a protected channel nobody can ever be admitted to.
    pub fn arm_creator_deadline(
        self: &Arc<Self>,
        group: String,
        on_expire: impl FnOnce() + Send + 'static,
    ) {
        let relay = Arc::clone(self);
        tokio::spawn(async move {
            tokio::time::sleep(Self::CREATOR_DEADLINE).await;
            if !relay.has_sockets(&group).await {
                on_expire();
            }
        });
    }

    fn body_len(frame: &GroupKeyFrame) -> usize {
        match frame {
            GroupKeyFrame::Broadcast(body)
            | GroupKeyFrame::Commit { body, .. }
            | GroupKeyFrame::Direct { body, .. }
            | GroupKeyFrame::Delivered { body, .. }
            | GroupKeyFrame::Asked { body, .. } => body.len(),
            GroupKeyFrame::Stale { .. }
            | GroupKeyFrame::Gone { .. }
            | GroupKeyFrame::Welcome { .. } => 0,
        }
    }
}

impl Default for GroupKeyRelayService {
    fn default() -> Self {
        Self::new()
    }
}
