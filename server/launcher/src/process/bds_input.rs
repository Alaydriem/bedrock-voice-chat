use std::fs::File;
use std::io::Write;
use std::os::fd::AsFd;
use std::process::ChildStdin;
use std::sync::{Arc, Mutex};
use std::thread;

/// The current BDS stdin pipe, shared by the console forwarder and the supervisor. Replaced when
/// BDS is restarted.
///
/// Writes go to a duplicate of the pipe, outside the lock: a BDS that stops reading blocks only
/// the writer, never the supervisor. The writer is freed when BDS is killed and the pipe breaks.
#[derive(Clone, Default)]
pub struct BdsInput {
    stdin: Arc<Mutex<Option<ChildStdin>>>,
}

impl BdsInput {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn attach(&self, stdin: ChildStdin) {
        if let Ok(mut current) = self.stdin.lock() {
            *current = Some(stdin);
        }
    }

    // A write to a BDS that has exited fails with a broken pipe; there is nobody to tell.
    pub fn write(&self, bytes: &[u8]) {
        if let Some(mut pipe) = self.pipe() {
            let _ = pipe.write_all(bytes);
        }
    }

    /// Sent from its own thread, so a full pipe cannot hold up the stop sequence's timeouts.
    pub fn send_stop(&self) {
        if let Some(mut pipe) = self.pipe() {
            thread::spawn(move || {
                let _ = pipe.write_all(b"stop\n");
            });
        }
    }

    fn pipe(&self) -> Option<File> {
        let current = self.stdin.lock().ok()?;
        let stdin = current.as_ref()?;
        stdin.as_fd().try_clone_to_owned().ok().map(File::from)
    }
}
