use std::sync::atomic::AtomicU32;

pub(super) struct RateLimitEntry {
    pub(super) count: AtomicU32,
}
