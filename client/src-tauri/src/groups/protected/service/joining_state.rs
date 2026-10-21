use bytes::Bytes;

use super::JoinPhase;

/// A joiner's progress, and the commits it must replay once it holds the group state: commits
/// accepted while its Welcome was in flight would otherwise be lost to it.
pub struct JoiningState {
    pub phase: JoinPhase,
    pub buffered_commits: Vec<(u64, Bytes)>,
}
