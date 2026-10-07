use std::ffi::OsString;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::time::{Duration, Instant};

use anyhow::{Context, anyhow};
use nix::sys::signal::{Signal, kill};
use nix::unistd::Pid;

use super::{BdsInput, ChildLock, IdleOutcome, ParentDeathSignal, SupervisorEvent};
use crate::Console;

/// Owns the BVC and BDS child processes. Must live on the main thread, which spawns every child
/// (see `ParentDeathSignal`).
pub struct Supervisor {
    events: Receiver<SupervisorEvent>,
    bds_input: BdsInput,
    child_lock: ChildLock,
    bds: Option<i32>,
    bvc: Option<i32>,
    bds_exit: Option<i32>,
    // Set once the launcher sends BVC its stop signal, so the exit that follows is expected.
    stopping_bvc: bool,
    bvc_failed: bool,
}

enum Wait {
    Done,
    TimedOut,
    Signalled,
}

impl Supervisor {
    const BDS_STOP_TIMEOUT: Duration = Duration::from_secs(20);
    const BVC_STOP_TIMEOUT: Duration = Duration::from_secs(10);
    const KILL_WAIT: Duration = Duration::from_secs(5);

    pub fn new(
        events: Receiver<SupervisorEvent>,
        bds_input: BdsInput,
        child_lock: ChildLock,
    ) -> Self {
        Self {
            events,
            bds_input,
            child_lock,
            bds: None,
            bvc: None,
            bds_exit: None,
            stopping_bvc: false,
            bvc_failed: false,
        }
    }

    pub fn has_children(&self) -> bool {
        self.bds.is_some() || self.bvc.is_some()
    }

    /// Whether BVC exited without the launcher stopping it.
    pub fn bvc_failed(&self) -> bool {
        self.bvc_failed
    }

    pub fn spawn_bvc(&mut self, binary: &Path, workdir: &Path) -> anyhow::Result<()> {
        let mut command = Command::new(binary);
        command
            .args(["-c", "config.hcl", "server"])
            .current_dir(workdir)
            .stdin(Stdio::null());
        ParentDeathSignal::apply(&mut command);
        let _guard = self.child_lock.hold();
        let child = command
            .spawn()
            .with_context(|| format!("starting {}", binary.display()))?;
        self.bvc = Some(child.id() as i32);
        self.stopping_bvc = false;
        self.bvc_failed = false;
        Ok(())
    }

    pub fn spawn_bds(
        &mut self,
        binary: &Path,
        workdir: &Path,
        args: &[OsString],
    ) -> anyhow::Result<()> {
        let mut command = Command::new(binary);
        command.args(args).current_dir(workdir).stdin(Stdio::piped());
        ParentDeathSignal::apply(&mut command);
        let _guard = self.child_lock.hold();
        let mut child = command
            .spawn()
            .with_context(|| format!("starting {}", binary.display()))?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| anyhow!("BDS started without a stdin pipe"))?;
        self.bds_input.attach(stdin);
        self.bds = Some(child.id() as i32);
        self.bds_exit = None;
        Ok(())
    }

    /// Runs until BDS exits or a signal arrives, then stops everything. Returns the launcher's
    /// exit code.
    pub fn run(&mut self) -> i32 {
        loop {
            match self.events.recv() {
                Ok(SupervisorEvent::Signal(_)) | Err(_) => return self.shutdown(),
                Ok(SupervisorEvent::Exited { pid, code }) => {
                    self.record_exit(pid, code);
                    if self.bds.is_none() {
                        self.stop_bvc();
                        return code;
                    }
                }
            }
        }
    }

    /// Whether a stop signal arrived without waiting for one. Exits that arrived with it are
    /// recorded. Checked before BDS starts, so a stop requested during setup never starts a BDS
    /// only to stop it while it creates a world.
    pub fn signal_pending(&mut self) -> bool {
        while let Ok(event) = self.events.try_recv() {
            match event {
                SupervisorEvent::Signal(_) => return true,
                SupervisorEvent::Exited { pid, code } => self.record_exit(pid, code),
            }
        }
        false
    }

    /// Waits while both children run normally.
    pub fn idle(&mut self, duration: Duration) -> IdleOutcome {
        let deadline = Instant::now() + duration;
        match self.wait_until(deadline, |s| s.bds.is_none()) {
            Wait::Done => IdleOutcome::BdsExited(self.bds_exit.unwrap_or(1)),
            Wait::TimedOut => IdleOutcome::Elapsed,
            Wait::Signalled => IdleOutcome::Signalled,
        }
    }

    /// Stops BDS so it can be started again; BVC keeps running. Returns true only once BDS has
    /// exited. False means a signal arrived or BDS outlived SIGKILL, and the caller must shut
    /// down rather than start a second BDS beside it.
    pub fn restart_bds_stop(&mut self) -> bool {
        self.bds_input.send_stop();
        match self.wait_until(Instant::now() + Self::BDS_STOP_TIMEOUT, |s| s.bds.is_none()) {
            Wait::Done => true,
            Wait::Signalled => false,
            Wait::TimedOut => {
                Self::send(self.bds, Signal::SIGKILL);
                matches!(
                    self.wait_until(Instant::now() + Self::KILL_WAIT, |s| s.bds.is_none()),
                    Wait::Done
                )
            }
        }
    }

    /// `stop` to BDS and SIGTERM to BVC at once, SIGKILL to whatever outlives its timeout. A
    /// second signal kills at once.
    pub fn shutdown(&mut self) -> i32 {
        self.bds_input.send_stop();
        self.stopping_bvc = true;
        Self::send(self.bvc, Signal::SIGTERM);
        let started = Instant::now();

        let mut forced = matches!(
            self.wait_until(started + Self::BVC_STOP_TIMEOUT, |s| s.bvc.is_none()),
            Wait::Signalled
        );
        if !forced {
            Self::send(self.bvc, Signal::SIGKILL);
            forced = matches!(
                self.wait_until(started + Self::BDS_STOP_TIMEOUT, |s| s.bds.is_none()),
                Wait::Signalled
            );
        }
        if forced {
            Console::warn("Second signal: killing BDS and BVC");
        }

        Self::send(self.bds, Signal::SIGKILL);
        Self::send(self.bvc, Signal::SIGKILL);
        let _ = self.wait_until(Instant::now() + Self::KILL_WAIT, |s| !s.has_children());
        self.bds_exit.unwrap_or(128 + Signal::SIGKILL as i32)
    }

    /// SIGTERM to BVC, SIGKILL after its timeout or on a signal.
    pub fn stop_bvc(&mut self) {
        self.stopping_bvc = true;
        Self::send(self.bvc, Signal::SIGTERM);
        let deadline = Instant::now() + Self::BVC_STOP_TIMEOUT;
        if !matches!(self.wait_until(deadline, |s| s.bvc.is_none()), Wait::Done) {
            Self::send(self.bvc, Signal::SIGKILL);
            let _ = self.wait_until(Instant::now() + Self::KILL_WAIT, |s| s.bvc.is_none());
        }
    }

    fn wait_until(&mut self, deadline: Instant, done: impl Fn(&Self) -> bool) -> Wait {
        loop {
            if done(self) {
                return Wait::Done;
            }
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Wait::TimedOut;
            }
            match self.events.recv_timeout(remaining) {
                Ok(SupervisorEvent::Signal(_)) => return Wait::Signalled,
                Ok(SupervisorEvent::Exited { pid, code }) => self.record_exit(pid, code),
                Err(RecvTimeoutError::Timeout) | Err(RecvTimeoutError::Disconnected) => {
                    return Wait::TimedOut;
                }
            }
        }
    }

    fn record_exit(&mut self, pid: i32, code: i32) {
        if self.bds == Some(pid) {
            self.bds = None;
            self.bds_exit = Some(code);
            Console::info(&format!("BDS exited with code {code}"));
        } else if self.bvc == Some(pid) {
            self.bvc = None;
            if self.stopping_bvc {
                Console::info(&format!("BVC stopped with code {code}"));
            } else if self.bds.is_some() {
                self.bvc_failed = true;
                Console::warn(&format!("BVC exited with code {code}. BDS keeps running."));
            } else {
                self.bvc_failed = true;
                Console::warn(&format!("BVC exited with code {code}"));
            }
        }
    }

    fn send(pid: Option<i32>, signal: Signal) {
        if let Some(pid) = pid {
            let _ = kill(Pid::from_raw(pid), signal);
        }
    }
}
