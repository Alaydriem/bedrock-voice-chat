use std::sync::{Arc, Mutex, MutexGuard};

/// Serialises spawning with reaping.
///
/// On a failed exec, `Command::spawn` waits on the child itself and panics if that wait fails.
/// A `waitpid(-1)` that collects the child first makes it fail, so the Reaper only reaps while
/// no spawn is in progress, and a spawn holds the lock until its pid is registered.
#[derive(Clone, Default)]
pub struct ChildLock {
    lock: Arc<Mutex<()>>,
}

impl ChildLock {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn hold(&self) -> MutexGuard<'_, ()> {
        self.lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}
