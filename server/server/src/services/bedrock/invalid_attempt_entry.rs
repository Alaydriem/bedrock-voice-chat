use std::sync::atomic::AtomicU32;

pub(super) struct InvalidAttemptEntry {
    pub(super) count: AtomicU32,
}
