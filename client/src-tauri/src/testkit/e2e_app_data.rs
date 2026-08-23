// Which app-data namespace an e2e client writes to.
//
// Tauri resolves `app_data_dir`, `app_local_data_dir` and `app_cache_dir` as
// `dirs::…()/identifier`, so the identifier is the whole of a client's isolation:
// two clients holding the same one share every file underneath it.
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
    // The identifier a harness run by hand falls back to, and the prefix every
    // namespace extends.
    pub const BASE_IDENTIFIER: &'static str = "com.alaydriem.bvc.client.e2e";

    // How the parent tells a spawned client which namespace to use.
    pub const ENV_VAR: &'static str = "BVC_E2E_APP_IDENTIFIER";

    // The namespace for one client of the calling process.
    //
    // Per gamertag as well as per process, so two clients in one scenario do not
    // write each other's `store.json`, and so a client that is shut down and
    // respawned within a scenario lands back on its own.
    pub fn namespace(gamertag: &str) -> String {
        format!(
            "{}.{}.{}",
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
    // Scoped to this process's own id, so it cannot touch a namespace another
    // test is using — which is the entire difference between this and the
    // recursive delete of the shared tree that it replaces. Reclaiming at startup
    // rather than at exit is what also collects the namespaces of a run that was
    // killed or panicked before it could clean up, since the operating system
    // hands the same process id out again.
    #[cfg(feature = "e2e")]
    pub fn reclaim_own() {
        let prefix = format!("{}.{}.", Self::BASE_IDENTIFIER, Self::process_tag());

        for base in [
            dirs::data_dir(),
            dirs::data_local_dir(),
            dirs::cache_dir(),
            dirs::config_dir(),
        ]
        .into_iter()
        .flatten()
        {
            let Ok(entries) = std::fs::read_dir(&base) else {
                continue;
            };

            for entry in entries.flatten() {
                if entry.file_name().to_string_lossy().starts_with(&prefix) {
                    let _ = std::fs::remove_dir_all(entry.path());
                }
            }
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
    // The leading `g` carries the empty gamertag, which would otherwise leave a
    // trailing `.` that Windows strips from a path component.
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
