use std::path::PathBuf;
use std::process::{Command, Stdio};

use super::AdminCommand;

/// The DB-direct `bvc-server` invocation for a console admin command.
pub struct AdminCli {
    binary: PathBuf,
    workdir: PathBuf,
    config: PathBuf,
}

impl AdminCli {
    pub fn new(binary: PathBuf, workdir: PathBuf, config: PathBuf) -> Self {
        Self {
            binary,
            workdir,
            config,
        }
    }

    pub fn is_installed(&self) -> bool {
        self.binary.exists()
    }

    pub fn command(&self, admin: &AdminCommand) -> Command {
        let mut command = Command::new(&self.binary);
        command
            .arg("-c")
            .arg(&self.config)
            .args(admin.args())
            .current_dir(&self.workdir)
            .stdin(Stdio::null());
        command
    }
}
