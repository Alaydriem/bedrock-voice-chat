use std::time::{Duration, Instant};

use common::structs::group_key::{GroupKeyFrame, SocketId};
use tokio::sync::mpsc;

use super::rate_verdict::RateVerdict;

/// One open key socket and its rate-limit windows.
pub struct SocketEntry {
    pub id: SocketId,
    pub identity: common::PlayerIdentity,
    pub tx: mpsc::Sender<GroupKeyFrame>,
    /// Has broadcast at least once. Members announce their binding on joining; joiners and
    /// strangers have nothing to broadcast, so they are never chosen to admit anyone.
    pub announced: bool,
    second_started: Option<Instant>,
    frames_this_second: u32,
    this_second_over: bool,
    window_started: Option<Instant>,
    bytes_this_window: usize,
    over_limit_since: Option<Instant>,
    last_members_frame: Option<Instant>,
}

impl SocketEntry {
    pub const MAX_FRAMES_PER_SEC: u32 = 20;
    pub const MAX_BYTES_PER_WINDOW: usize = 1024 * 1024;
    pub const WINDOW: Duration = Duration::from_secs(10);
    pub const OVER_LIMIT_CLOSE: Duration = Duration::from_secs(30);
    pub const MEMBERS_INTERVAL: Duration = Duration::from_secs(10);

    pub fn new(
        id: SocketId,
        identity: common::PlayerIdentity,
        tx: mpsc::Sender<GroupKeyFrame>,
    ) -> Self {
        Self {
            id,
            identity,
            tx,
            announced: false,
            second_started: None,
            frames_this_second: 0,
            this_second_over: false,
            window_started: None,
            bytes_this_window: 0,
            over_limit_since: None,
            last_members_frame: None,
        }
    }

    /// A frame to every member puts a prompt on every member's screen, so a socket may send
    /// one per [`Self::MEMBERS_INTERVAL`].
    pub fn permits_members_frame(&mut self, now: Instant) -> bool {
        let permitted = self
            .last_members_frame
            .is_none_or(|last| now.duration_since(last) >= Self::MEMBERS_INTERVAL);
        if permitted {
            self.last_members_frame = Some(now);
        }
        permitted
    }

    pub fn meter(&mut self, bytes: usize, now: Instant) -> RateVerdict {
        if self
            .second_started
            .is_none_or(|start| now.duration_since(start) >= Duration::from_secs(1))
        {
            // The grace clock only resets after a whole second within the limits: the first
            // frames of every second are always within, and must not excuse a socket that
            // exceeds the limit second after second.
            if !self.this_second_over {
                self.over_limit_since = None;
            }
            self.second_started = Some(now);
            self.frames_this_second = 0;
            self.this_second_over = false;
        }
        if self
            .window_started
            .is_none_or(|start| now.duration_since(start) >= Self::WINDOW)
        {
            self.window_started = Some(now);
            self.bytes_this_window = 0;
        }
        self.frames_this_second += 1;
        self.bytes_this_window += bytes;

        let over = self.frames_this_second > Self::MAX_FRAMES_PER_SEC
            || self.bytes_this_window > Self::MAX_BYTES_PER_WINDOW;
        if !over {
            return RateVerdict::Within;
        }

        self.this_second_over = true;
        let since = *self.over_limit_since.get_or_insert(now);
        if now.duration_since(since) >= Self::OVER_LIMIT_CLOSE {
            RateVerdict::Close
        } else {
            RateVerdict::Over
        }
    }
}
