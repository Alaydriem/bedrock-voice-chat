use std::sync::mpsc::Sender;
use std::thread;
use std::time::Duration;

use nix::errno::Errno;
use nix::sys::wait::{WaitPidFlag, WaitStatus, waitpid};

use super::{ChildLock, SupervisorEvent};

/// Collects every exited child with `waitpid(-1)`. This also reaps orphans re-parented to the
/// launcher when it runs as PID 1.
pub struct Reaper;

impl Reaper {
    const IDLE: Duration = Duration::from_millis(50);

    pub fn spawn(events: Sender<SupervisorEvent>, child_lock: ChildLock) {
        thread::spawn(move || {
            loop {
                let exited = Self::reap_exited(&child_lock);
                for event in exited {
                    if events.send(event).is_err() {
                        return;
                    }
                }
                thread::sleep(Self::IDLE);
            }
        });
    }

    // Non-blocking, so the lock is held only while children are collected.
    fn reap_exited(child_lock: &ChildLock) -> Vec<SupervisorEvent> {
        let _guard = child_lock.hold();
        let mut exited = Vec::new();
        loop {
            match waitpid(None, Some(WaitPidFlag::WNOHANG)) {
                Ok(WaitStatus::Exited(pid, code)) => exited.push(SupervisorEvent::Exited {
                    pid: pid.as_raw(),
                    code,
                }),
                Ok(WaitStatus::Signaled(pid, signal, _)) => exited.push(SupervisorEvent::Exited {
                    pid: pid.as_raw(),
                    code: 128 + signal as i32,
                }),
                Ok(WaitStatus::StillAlive) | Err(Errno::ECHILD) => return exited,
                Ok(_) | Err(Errno::EINTR) => continue,
                Err(_) => return exited,
            }
        }
    }
}
