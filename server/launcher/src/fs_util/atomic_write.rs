use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::Context;

/// Writes a file so a reader sees either the old content or the new, never a partial file, and
/// so the new content is on disk before the name points at it.
pub struct AtomicWrite;

impl AtomicWrite {
    pub fn write(path: &Path, bytes: &[u8]) -> anyhow::Result<()> {
        let temp = Self::stage(path, bytes)?;
        Self::commit(&temp, path)
    }

    /// Writes and syncs `bytes` beside `path`, without replacing it. Returns the staged file
    /// for `commit`.
    pub fn stage(path: &Path, bytes: &[u8]) -> anyhow::Result<PathBuf> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("creating {}", parent.display()))?;
        }
        let temp = Self::temp_path(path);
        let result = File::create(&temp)
            .and_then(|mut file| file.write_all(bytes).and_then(|()| file.sync_all()));
        if let Err(e) = result {
            let _ = fs::remove_file(&temp);
            return Err(e).with_context(|| format!("writing {}", temp.display()));
        }
        Ok(temp)
    }

    /// Moves a staged file into place.
    pub fn commit(temp: &Path, path: &Path) -> anyhow::Result<()> {
        fs::rename(temp, path)
            .with_context(|| format!("moving {} to {}", temp.display(), path.display()))?;
        Self::sync_parent(path);
        Ok(())
    }

    /// Removes a staged file that will not be committed.
    pub fn discard(temp: &Path) {
        let _ = fs::remove_file(temp);
    }

    #[cfg(unix)]
    pub fn mark_executable(path: &Path) -> anyhow::Result<()> {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o755))
            .with_context(|| format!("making {} executable", path.display()))
    }

    #[cfg(not(unix))]
    pub fn mark_executable(_path: &Path) -> anyhow::Result<()> {
        Ok(())
    }

    fn temp_path(path: &Path) -> PathBuf {
        let mut name = path.as_os_str().to_owned();
        name.push(".tmp");
        PathBuf::from(name)
    }

    // Makes the rename itself durable. Best effort: a failure here leaves a correct file that a
    // power loss might roll back to the old one.
    #[cfg(unix)]
    fn sync_parent(path: &Path) {
        if let Some(parent) = path.parent()
            && let Ok(dir) = File::open(parent)
        {
            let _ = dir.sync_all();
        }
    }

    #[cfg(not(unix))]
    fn sync_parent(_path: &Path) {}
}
