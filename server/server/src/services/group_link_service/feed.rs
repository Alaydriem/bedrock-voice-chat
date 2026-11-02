use std::collections::{HashSet, VecDeque};
use std::sync::{Arc, Mutex};

use common::structs::control::{GroupLinkDrain, GroupLinkEvent, GroupLinkEventKind};
use rand::RngExt;

/// BVC channel changes, kept for the Simple Voice Chat bridge to drain.
///
/// A pull rather than a push, so the embedded mod reads it over FFI and an external one over
/// HTTP with the same sequence numbers. The ring is bounded: a bridge that falls further behind
/// than it holds is told to resync rather than handed a partial history.
///
/// Nothing about a protected channel reaches the bridge. A channel created protected is silenced
/// before its first event; one that becomes protected publishes `Protected` and is silenced
/// after it. A `Delete` releases the silence, so a later channel with the same id starts clean.
pub struct GroupLinkFeed {
    state: Mutex<FeedState>,
    capacity: usize,
    epoch: u64,
}

struct FeedState {
    next_seq: u64,
    events: VecDeque<GroupLinkEvent>,
    silenced: HashSet<String>,
}

impl GroupLinkFeed {
    /// About four minutes of the busiest realistic server at the bridge's 250 ms drain.
    pub const CAPACITY: usize = 1024;

    pub fn new(capacity: usize) -> Self {
        let capacity = capacity.max(1);
        Self {
            state: Mutex::new(FeedState {
                next_seq: 1,
                events: VecDeque::with_capacity(capacity),
                silenced: HashSet::new(),
            }),
            capacity,
            // 52 random bits with the low one set: never zero, and exact as a JSON number.
            epoch: (rand::rng().random::<u64>() >> 12) | 1,
        }
    }

    pub fn new_shared(capacity: usize) -> Arc<Self> {
        Arc::new(Self::new(capacity))
    }

    pub fn publish(
        &self,
        kind: GroupLinkEventKind,
        channel: &str,
        channel_name: Option<String>,
        player: Option<String>,
        members_only: bool,
    ) {
        let mut state = self.state.lock().expect("group link feed lock");

        if state.silenced.contains(channel) {
            if kind == GroupLinkEventKind::Delete {
                state.silenced.remove(channel);
            }
            return;
        }
        if kind == GroupLinkEventKind::Protected {
            state.silenced.insert(channel.to_string());
        }

        let seq = state.next_seq;
        state.next_seq += 1;
        if state.events.len() == self.capacity {
            state.events.pop_front();
        }
        state.events.push_back(GroupLinkEvent {
            seq,
            kind,
            channel: channel.to_string(),
            channel_name,
            player,
            members_only,
        });
    }

    /// For a channel created protected: it publishes nothing until it is deleted.
    pub fn silence(&self, channel: &str) {
        self.state
            .lock()
            .expect("group link feed lock")
            .silenced
            .insert(channel.to_string());
    }

    /// Every kept event after `after`. `reset` when an event after `after` was dropped, or
    /// when `after` is ahead of the feed because the server restarted.
    pub fn drain(&self, after: u64) -> GroupLinkDrain {
        let state = self.state.lock().expect("group link feed lock");
        let latest = state.next_seq - 1;
        let evicted = state
            .events
            .front()
            .is_some_and(|oldest| oldest.seq > after.saturating_add(1));
        let reset = after > latest || evicted;

        let events = state
            .events
            .iter()
            .filter(|event| reset || event.seq > after)
            .cloned()
            .collect();

        GroupLinkDrain {
            events,
            reset,
            epoch: self.epoch,
        }
    }
}
