use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use common::structs::audio::{PlayerGainSettings, PlayerGainStore};
use common::structs::players::{PlayerKey, PlayerSettingsRow};
use log::{error, warn};

use super::backend::{MemoryBackend, PlayerSettings, PlayerSettingsBackend};

/// Per-player volume and mute, keyed by `(server, game:gamertag)`.
///
/// The in-memory map is authoritative and serves every read, so the settings pane's search,
/// filter, sort and paging need no query engine. Writes split by durability: a decision the
/// user made is written through at once, while a proximity `last_seen` stamp only marks the
/// map dirty and is flushed on a debounce. Somebody walking into earshot must not cost a disk
/// write, and proximity produces one of those per player who walks past.
pub struct PlayerSettingsService {
    /// Where the rows are kept. `MemoryBackend` when the file could not be opened, which is a
    /// backend rather than an absence — the service does not branch on it.
    backend: PlayerSettings,
    settings: RwLock<HashMap<PlayerKey, PlayerGainSettings>>,
    dirty: AtomicBool,
    /// Counts `touch` calls, so the debounce can tell "still arriving" from "gone quiet".
    /// A count rather than a timestamp: it needs no clock and cannot go backwards.
    stamps: AtomicU64,
}

impl PlayerSettingsService {
    /// Rows nobody decided anything about expire after this long.
    ///
    /// Thirty days is well past "I recognise that name" and well short of letting a busy
    /// server's traffic accumulate indefinitely. Rows carrying a decision are never subject
    /// to it.
    pub const STALE_AFTER: Duration = Duration::from_secs(30 * 24 * 60 * 60);

    pub fn new_shared(backend: PlayerSettings) -> Arc<Self> {
        let loaded: HashMap<PlayerKey, PlayerGainSettings> = match backend.load_all() {
            Ok(rows) => rows
                .into_iter()
                .map(|row| (row.key, row.settings))
                .collect(),
            Err(cause) => {
                error!("Could not load player settings, starting empty: {cause}");
                HashMap::new()
            }
        };

        let before = loaded.len();
        let settings = Self::pruned(loaded, Self::STALE_AFTER);
        let dropped = before - settings.len();

        // A load that could not decode some row must never be written back. Every write is a
        // whole-file rewrite, so persisting a partial load deletes exactly the rows this build
        // failed to understand — which are the ones a newer build wrote, or the ones a field
        // added without a serde default broke. Keep them on disk and let a human look.
        let partial = backend.skipped_rows();
        let service = Arc::new(Self {
            backend,
            settings: RwLock::new(settings),
            dirty: AtomicBool::new(false),
            stamps: AtomicU64::new(0),
        });

        if partial {
            warn!(
                "Player settings loaded with undecodable rows; leaving the file alone this session"
            );
        } else if dropped > 0 {
            // Only when the prune actually removed something, so an ordinary launch does not
            // rewrite the whole file for nothing. A failure costs nothing — the map in memory
            // is already pruned and the rows are merely stale, not wrong.
            if let Err(cause) = service.persist() {
                warn!("Could not write back pruned player settings: {cause}");
            }
        }
        service
    }

    /// Drops rows that record only that somebody walked past, long enough ago to have been
    /// forgotten.
    ///
    /// A row carrying a decision is kept regardless of age, and regardless of whether it was
    /// ever stamped: a mute set on somebody you never walked past has no `last_seen` at all,
    /// and treating an absent stamp as infinitely old would delete exactly the rows the user
    /// cared most about. `is_adjusted` is reused rather than re-derived here — a second copy
    /// of that rule is how a pruner starts deleting mutes.
    fn pruned(
        rows: HashMap<PlayerKey, PlayerGainSettings>,
        older_than: Duration,
    ) -> HashMap<PlayerKey, PlayerGainSettings> {
        let now = Self::now_millis();
        let cutoff = now - older_than.as_millis() as f64;

        rows.into_iter()
            .filter(|(_, settings)| {
                settings.is_adjusted() || settings.last_seen.is_some_and(|seen| seen >= cutoff)
            })
            .collect()
    }

    /// A service with nothing behind it, for when the file could not be opened.
    pub fn new_memory_only() -> Arc<Self> {
        Arc::new(Self {
            backend: PlayerSettings::Memory(MemoryBackend::new()),
            settings: RwLock::new(HashMap::new()),
            dirty: AtomicBool::new(false),
            stamps: AtomicU64::new(0),
        })
    }

    /// What the mixer should do about this player. Unity gain when nothing is known, so a
    /// missing row plays the speaker rather than dropping them.
    pub fn get(&self, key: &PlayerKey) -> PlayerGainSettings {
        self.settings
            .read()
            .ok()
            .and_then(|settings| settings.get(key).cloned())
            .unwrap_or_else(PlayerGainSettings::unity)
    }

    pub fn set_gain(&self, key: &PlayerKey, gain: f32) -> Result<(), anyhow::Error> {
        self.mutate(key, |settings| settings.gain = gain);
        self.persist()
    }

    pub fn set_muted(&self, key: &PlayerKey, muted: bool) -> Result<(), anyhow::Error> {
        self.mutate(key, |settings| settings.muted = muted);
        self.persist()
    }

    /// Records that this player was near you just now.
    ///
    /// Deliberately does not persist. Proximity stamps arrive once per player who walks past,
    /// and a stamp is worth almost nothing individually — the debounce in `spawn_debounce`
    /// coalesces a crowd into one write.
    pub fn touch(&self, key: &PlayerKey) {
        self.mutate(key, |settings| settings.last_seen = Some(Self::now_millis()));
        self.stamps.fetch_add(1, Ordering::Relaxed);
        self.dirty.store(true, Ordering::Relaxed);
    }

    pub fn forget(&self, key: &PlayerKey) -> Result<(), anyhow::Error> {
        if let Ok(mut settings) = self.settings.write() {
            settings.remove(key);
        }
        self.persist()
    }

    /// Puts one server's players back to unity gain and unmuted, keeping the rows.
    ///
    /// Scoped to a server because the user is looking at one server's list and cannot see the
    /// effect on any other. `last_seen` survives, so the list keeps its order and does not
    /// empty itself.
    pub fn reset_all(&self, server: &str) -> Result<(), anyhow::Error> {
        if let Ok(mut settings) = self.settings.write() {
            for (key, value) in settings.iter_mut() {
                if key.server == server {
                    value.gain = 1.0;
                    value.muted = false;
                }
            }
        }
        self.persist()
    }

    /// One server's rows, for the settings pane.
    pub fn rows(&self, server: &str) -> Vec<PlayerSettingsRow> {
        let Ok(settings) = self.settings.read() else {
            return Vec::new();
        };
        settings
            .iter()
            .filter(|(key, _)| key.server == server)
            .map(|(key, value)| PlayerSettingsRow::new(key.clone(), value.clone()))
            .collect()
    }

    /// How many players on this server the user has muted, for the diagnostics report.
    ///
    /// Scoped to the server for the same reason everything else here is: a mute on a server
    /// you are not connected to is not silencing anybody in this session, and reporting it
    /// would make the diagnostics contradict what the user can hear.
    pub fn muted_count(&self, server: &str) -> u32 {
        let Ok(settings) = self.settings.read() else {
            return 0;
        };
        settings
            .iter()
            .filter(|(key, value)| key.server == server && value.muted)
            .count() as u32
    }

    /// One server's rows in the identity-keyed shape the mixer consumes.
    ///
    /// The only place the server is dropped from the key. `GainProjection` has no server
    /// dimension, so if this flattening happened anywhere else — or without the filter — one
    /// server's mute would silence that player on every server.
    pub fn store_for(&self, server: &str) -> PlayerGainStore {
        let Ok(settings) = self.settings.read() else {
            return PlayerGainStore::default();
        };
        PlayerGainStore(
            settings
                .iter()
                .filter(|(key, _)| key.server == server)
                .map(|(key, value)| (key.cn.clone(), value.clone()))
                .collect(),
        )
    }

    /// Persists pending `touch`es. A no-op when nothing is dirty or there is no file.
    pub fn flush(&self) -> Result<(), anyhow::Error> {
        if !self.dirty.load(Ordering::Relaxed) {
            return Ok(());
        }
        self.persist()
    }

    pub fn dirty(&self) -> bool {
        self.dirty.load(Ordering::Relaxed)
    }

    /// How often the debounce wakes, and how many quiet wakeups mean the burst has ended.
    /// Two seconds of quiet, checked four times.
    const DEBOUNCE_TICK: Duration = Duration::from_millis(500);
    const QUIET_TICKS_TO_FLUSH: u32 = 4;

    /// Flushes pending proximity stamps once no new stamp has arrived for four ticks.
    ///
    /// Quiet-based rather than periodic: walking through a crowded area produces a stamp per
    /// player, and the point is to turn that burst into one write after it ends rather than a
    /// write every two seconds throughout it.
    ///
    /// `tauri::async_runtime::spawn`, not `tokio::spawn`: this is called from the synchronous
    /// `build_managed_state`, where there is no ambient Tokio runtime and a bare `tokio::spawn`
    /// panics the whole setup.
    pub fn spawn_debounce(self: Arc<Self>) {
        tauri::async_runtime::spawn(async move {
            let mut last_count = 0_u64;
            let mut quiet_ticks = 0_u32;

            loop {
                tokio::time::sleep(Self::DEBOUNCE_TICK).await;

                if !self.dirty() {
                    quiet_ticks = 0;
                    continue;
                }

                let count = self.stamps.load(Ordering::Relaxed);
                if count != last_count {
                    last_count = count;
                    quiet_ticks = 0;
                    continue;
                }

                quiet_ticks += 1;
                if quiet_ticks >= Self::QUIET_TICKS_TO_FLUSH {
                    if let Err(cause) = self.flush() {
                        error!("Could not flush player settings: {cause}");
                    }
                    quiet_ticks = 0;
                }
            }
        });
    }

    fn mutate(&self, key: &PlayerKey, change: impl FnOnce(&mut PlayerGainSettings)) {
        let Ok(mut settings) = self.settings.write() else {
            warn!("Player settings lock is poisoned; dropping a change for {}", key.encode());
            return;
        };
        let entry = settings
            .entry(key.clone())
            .or_insert_with(PlayerGainSettings::unity);
        change(entry);
    }

    /// Writes the whole map in one transaction and clears the dirty flag.
    ///
    /// A full rewrite per decision rather than a diff. At this cardinality it costs
    /// microseconds, and it leaves no partial-application state for the file and the map to
    /// disagree about.
    ///
    /// The flag is cleared BEFORE the snapshot, not after. A `touch` landing between the
    /// snapshot and the clear would otherwise have its flag wiped by a write that did not
    /// contain it — the stamp would sit in memory, marked clean, and be lost at exit. Clearing
    /// first can only ever cost one redundant write.
    fn persist(&self) -> Result<(), anyhow::Error> {
        self.dirty.store(false, Ordering::Relaxed);

        // The read guard is deliberately held ACROSS the write, not dropped after the
        // snapshot. Dropping it first lets a mutation land in between, so two concurrent
        // persists can reach `write_all` in the opposite order to their snapshots and leave
        // the older one on disk — with `dirty` already clear, nothing would ever repair it and
        // memory would disagree with the file for the rest of the session. Holding it blocks
        // other *writers* for one small transaction; readers are unaffected, which is what the
        // audio path needs.
        let Ok(settings) = self.settings.read() else {
            self.dirty.store(true, Ordering::Relaxed);
            return Err(anyhow::anyhow!("player settings lock is poisoned"));
        };
        let rows: Vec<PlayerSettingsRow> = settings
            .iter()
            .map(|(key, value)| PlayerSettingsRow::new(key.clone(), value.clone()))
            .collect();

        // A failed write re-arms the flag so the debounce retries, rather than leaving the map
        // marked clean with its contents never written.
        if let Err(cause) = self.backend.write_all(&rows) {
            self.dirty.store(true, Ordering::Relaxed);
            return Err(cause);
        }
        Ok(())
    }

    fn now_millis() -> f64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|since| since.as_millis() as f64)
            .unwrap_or_default()
    }
}
