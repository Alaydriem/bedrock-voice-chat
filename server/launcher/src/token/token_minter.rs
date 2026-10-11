use std::path::PathBuf;
use std::process::Command;
use std::thread;
use std::time::Duration;

use anyhow::bail;

/// Issues a game access token by running BVC's own `admin token mint --local`.
pub struct TokenMinter {
    binary: PathBuf,
    workdir: PathBuf,
    attempts: u32,
    delay: Duration,
}

impl TokenMinter {
    const ARGS: [&'static str; 6] = ["-c", "config.hcl", "admin", "token", "mint", "--local"];

    pub fn new(binary: PathBuf, workdir: PathBuf, attempts: u32, delay: Duration) -> Self {
        Self {
            binary,
            workdir,
            attempts,
            delay,
        }
    }

    /// Retries while BVC is still creating its tables. The last attempt's error is returned.
    pub fn mint(&self) -> anyhow::Result<String> {
        let mut last_error = String::from("no attempt was made");
        for attempt in 1..=self.attempts {
            match self.attempt() {
                Ok(token) => return Ok(token),
                Err(error) => last_error = error,
            }
            if attempt < self.attempts {
                thread::sleep(self.delay);
            }
        }
        bail!(
            "minting a token failed after {} attempts: {last_error}",
            self.attempts
        )
    }

    fn attempt(&self) -> Result<String, String> {
        let output = Command::new(&self.binary)
            .args(Self::ARGS)
            .current_dir(&self.workdir)
            .output()
            .map_err(|e| e.to_string())?;
        if !output.status.success() {
            return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
        }
        let token = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if token.is_empty() {
            return Err("the mint command printed no token".to_string());
        }
        Ok(token)
    }
}
