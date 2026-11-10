use std::collections::{HashSet, VecDeque};

use common::structs::control::GroupLinkEvent;

/// What the feed holds under its lock: the events it still keeps, the number the next one
/// takes, and the channels that publish nothing.
pub(super) struct FeedState {
    pub(super) next_seq: u64,
    pub(super) events: VecDeque<GroupLinkEvent>,
    pub(super) silenced: HashSet<String>,
}

impl FeedState {
    pub(super) fn new(capacity: usize) -> Self {
        Self {
            next_seq: 1,
            events: VecDeque::with_capacity(capacity),
            silenced: HashSet::new(),
        }
    }
}
