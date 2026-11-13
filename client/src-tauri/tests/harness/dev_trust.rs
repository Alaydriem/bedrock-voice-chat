use std::path::PathBuf;
use std::sync::Mutex;

use common::rustls::DevTrustRoots;

// The CA of every harness server this process has booted, in boot order.
static ROOTS: Mutex<Vec<String>> = Mutex::new(Vec::new());

/// The trust a development client needs to reach the harness servers, which sign their own
/// certificates. The bundle lives in a file this process owns, and `BVC_DEV_TRUST_ROOTS` names
/// it. Setting the variable on the test process covers the in-process client library, and the
/// e2e client binaries it spawns inherit it.
pub struct DevTrust;

impl DevTrust {
    /// Adds a server's CA to the bundle and points `BVC_DEV_TRUST_ROOTS` at it.
    pub fn register(ca_pem: &str) {
        let mut roots = ROOTS.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        if !roots.iter().any(|known| known == ca_pem) {
            roots.push(ca_pem.to_string());
        }

        let path = Self::bundle_path();
        Self::replace(&path, &roots.join("\n"));
        // Held under `ROOTS`, so concurrent registrations do not interleave.
        unsafe { std::env::set_var(DevTrustRoots::ENV, &path) };
    }

    /// Drops a server's CA from the bundle. When no server is left the variable is unset and
    /// the last bundle stays on disk: a client process spawned earlier still holds the path,
    /// and a missing file would fail its TLS setup. The file is named for this process and is
    /// replaced by the next registration.
    pub fn unregister(ca_pem: &str) {
        let mut roots = ROOTS.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        roots.retain(|known| known != ca_pem);

        let path = Self::bundle_path();
        if roots.is_empty() {
            unsafe { std::env::remove_var(DevTrustRoots::ENV) };
            let _ = std::fs::remove_file(path.with_extension("pem.next"));
        } else {
            Self::replace(&path, &roots.join("\n"));
        }
    }

    // A client may build its TLS configuration while a later server registers. Writing in
    // place would let it read a truncated bundle, so the new bundle is written beside the
    // old one and renamed over it. On Windows the rename fails while a reader holds the file
    // open, so it is retried briefly.
    fn replace(path: &PathBuf, contents: &str) {
        let staged = path.with_extension("pem.next");
        std::fs::write(&staged, contents).expect("write the staged dev trust bundle");

        let mut last_error = None;
        for _ in 0..50 {
            match std::fs::rename(&staged, path) {
                Ok(()) => return,
                Err(e) => {
                    last_error = Some(e);
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
            }
        }
        panic!("replace the dev trust bundle: {last_error:?}");
    }

    fn bundle_path() -> PathBuf {
        std::env::temp_dir().join(format!("bvc-e2e-dev-trust-{}.pem", std::process::id()))
    }
}
