use common::structs::players::PlayerSettingsRow;

use super::PlayerSettingsBackend;

/// Settings with nothing behind them, for a session whose file could not be opened.
///
/// A real backend rather than an absent one. The condition it covers — the file locked by
/// another instance, a permission error, an antivirus scan — is transient, so the right response
/// is to keep working for this session and leave the file alone, not to fail startup and not to
/// overwrite whatever is there. Audio still applies whatever the user sets; it simply does not
/// outlive the process.
///
/// Writes succeed rather than erroring. The caller has no better option to offer the user, and
/// an error per slider movement would be noise about a decision already made at startup.
#[derive(Default)]
pub struct MemoryBackend;

impl MemoryBackend {
    pub fn new() -> Self {
        Self
    }
}

impl PlayerSettingsBackend for MemoryBackend {
    fn load_all(&self) -> Result<Vec<PlayerSettingsRow>, anyhow::Error> {
        Ok(Vec::new())
    }

    fn write_all(&self, _rows: &[PlayerSettingsRow]) -> Result<(), anyhow::Error> {
        Ok(())
    }

    fn skipped_rows(&self) -> bool {
        false
    }
}
