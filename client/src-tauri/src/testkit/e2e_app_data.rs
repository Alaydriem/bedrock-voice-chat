// Which app-data namespace an e2e client writes to.
//
// Tauri resolves `app_data_dir`, `app_local_data_dir` and `app_cache_dir` by
// joining the identifier onto a base directory, so the identifier is the whole of
// a client's isolation: two clients holding the same one share every file
// underneath it. `PathBuf::join` reads `/` as a separator on Windows as well as
// Unix, so an identifier carrying one resolves to a nested tree rather than a
// single directory named after the whole string.
//
// Every e2e client used to hold one constant identifier, and the harness deleted
// that shared tree before each spawn. Under nextest — one process per test — the
// deletes ran concurrently with other tests' live clients, and one landing inside
// `tauri_plugin_http`'s create-directory-then-open-cookie-file window aborted the
// client with `PluginInitialization("http", os error 3)`. The test that owned it
// then failed on a connect timeout, so the damage always surfaced on a different
// test than the one that caused it.
//
// The namespace is keyed on the process id for that reason. Two live processes
// cannot share one, which is what makes reclaiming a namespace safe: a process
// only ever deletes its own.
pub struct E2eAppData;

impl E2eAppData {
    // The identifier a harness run by hand falls back to, and the root every
    // namespace nests under.
    pub const BASE_IDENTIFIER: &'static str = "com.alaydriem.bvc.client.e2e";

    // How the parent tells a spawned client which namespace to use.
    pub const ENV_VAR: &'static str = "BVC_E2E_APP_IDENTIFIER";

    // The namespace for one client of the calling process.
    //
    // Per gamertag as well as per process, so two clients in one scenario do not
    // write each other's `store.json`, and so a client that is shut down and
    // respawned within a scenario lands back on its own.
    //
    // Three path components rather than one dotted name: the base directories
    // hold one entry per suite instead of one per process per gamertag, and a
    // whole process's clients are reclaimed by removing a single directory.
    pub fn namespace(gamertag: &str) -> String {
        format!(
            "{}/{}/{}",
            Self::BASE_IDENTIFIER,
            Self::process_tag(),
            Self::encode(gamertag)
        )
    }

    // The identifier the spawned client applies to its Tauri context.
    //
    // Falls back rather than failing: the harness bin is also run by hand without
    // a parent, and the fallback still keeps it out of the real client's app-data.
    pub fn identifier() -> String {
        std::env::var(Self::ENV_VAR)
            .ok()
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| Self::BASE_IDENTIFIER.to_string())
    }

    // Deletes the namespaces belonging to the calling process.
    //
    // Removes this process's own directory, which holds one subdirectory per
    // gamertag and nothing another test can be using — the entire difference
    // between this and the recursive delete of the shared tree that it replaces.
    // Reclaiming at startup rather than at exit is what also collects the
    // namespaces of a run that was killed or panicked before it could clean up,
    // since the operating system hands the same process id out again.
    #[cfg(feature = "e2e")]
    pub fn reclaim_own() {
        for base in [
            dirs::data_dir(),
            dirs::data_local_dir(),
            dirs::cache_dir(),
            dirs::config_dir(),
        ]
        .into_iter()
        .flatten()
        {
            let own = base.join(Self::BASE_IDENTIFIER).join(Self::process_tag());

            let _ = std::fs::remove_dir_all(own);
        }
    }

    fn process_tag() -> String {
        format!("p{}", std::process::id())
    }

    // Renders a gamertag as one path component without merging two tags onto one.
    //
    // A gamertag is not path-safe: the scenarios use an empty one, and nothing
    // stops one carrying a separator. Mapping every unsafe character to the same
    // replacement would collide `a/b` with `a b`, putting two clients back on one
    // namespace — the defect this type exists to remove, reintroduced quietly.
    // Escaping is reversible instead, so distinct tags stay distinct.
    //
    // The leading `g` carries the empty gamertag, which would otherwise contribute
    // an empty component and collapse the namespace onto the process directory.
    fn encode(gamertag: &str) -> String {
        let mut out = String::from("g");

        for byte in gamertag.bytes() {
            if byte.is_ascii_alphanumeric() {
                out.push(byte as char);
            } else {
                out.push_str(&format!("_{byte:02X}"));
            }
        }

        out
    }
}
