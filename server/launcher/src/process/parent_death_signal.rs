use std::io;
use std::os::unix::process::CommandExt;
use std::process::Command;

/// Makes the kernel SIGKILL a child when the launcher dies, including when the launcher itself
/// is SIGKILLed and runs no cleanup.
///
/// The signal is tied to the thread that spawns the child, not the process, so children must be
/// spawned from a thread that lives as long as the launcher.
pub struct ParentDeathSignal;

impl ParentDeathSignal {
    pub fn apply(command: &mut Command) {
        let parent = std::process::id() as libc::pid_t;
        // Only async-signal-safe calls between fork and exec: no allocation, no locks.
        unsafe {
            command.pre_exec(move || {
                if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL, 0, 0, 0) != 0 {
                    return Err(io::Error::last_os_error());
                }
                // The launcher died between fork and prctl; the signal will never come.
                if libc::getppid() != parent {
                    return Err(io::Error::from_raw_os_error(libc::ESRCH));
                }
                Ok(())
            });
        }
    }
}
