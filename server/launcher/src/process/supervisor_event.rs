use super::AdminCommand;

/// What the supervisor waits on. `code` is the exit code, or 128 plus the signal number for a
/// child killed by a signal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SupervisorEvent {
    Signal(i32),
    Exited { pid: i32, code: i32 },
    /// A `bvc` line typed into the panel console.
    Command(AdminCommand),
}
