use std::time::Instant;

/// The time source the key service reads, so tests can drive its timers.
pub trait Clock: Send + Sync + 'static {
    fn now(&self) -> Instant;
}
