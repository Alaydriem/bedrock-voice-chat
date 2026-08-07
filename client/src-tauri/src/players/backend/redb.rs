use std::io::ErrorKind;
use std::sync::atomic::{AtomicBool, Ordering};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Context;
use common::structs::audio::PlayerGainSettings;
use common::structs::players::{PlayerKey, PlayerSettingsRow};

use super::PlayerSettingsBackend;
use log::{error, warn};
use redb::{
    Database, DatabaseError, ReadableDatabase, ReadableTable, StorageError, TableDefinition,
};

/// One table. The key is `PlayerKey::encode()` and the value is JSON-encoded
/// `PlayerGainSettings`.
///
/// A composite string key rather than a redb tuple key, so the encoding is ours and explicit
/// and `PlayerKey::server_prefix` is a plain range scan.
///
/// JSON rather than postcard for the value, despite postcard being this codebase's default
/// binary encoding. `PlayerGainSettings::last_seen` carries `skip_serializing_if`, which
/// omits the field entirely when it is `None`. A self-describing format reads that back as
/// the default; postcard, which encodes a bare field sequence with no names, runs off the end
/// of the buffer instead — so every row belonging to a player the user muted but never walked
/// past would fail to decode and be silently dropped at load. The store is a few kilobytes
/// read once at startup, so the size difference is not worth that class of bug.
const TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("player_settings");

/// The durable half of the player settings store.
///
/// Owns the redb file and nothing else. The in-memory map that serves reads lives in
/// `PlayerSettingsService`; this type only moves whole rows in and out.
pub struct RedbBackend {
    db: Database,
    /// Set when a load could not decode a row, so the caller can refuse to write back over it.
    skipped: AtomicBool,
}

impl RedbBackend {
    /// Opens the database, separating three outcomes. It never deletes a file.
    ///
    /// 1. Opened. The normal case, including redb's own repair of an unclean shutdown.
    /// 2. Cannot be opened — locked, no permission, IO error, disk full. Transient, so this
    ///    returns `Err` and the caller runs in memory for the session. The file is untouched.
    /// 3. Opened but unreadable, or from a version this build cannot read. The file is renamed
    ///    to `<name>.corrupt-<unix-seconds>` and a new database is created beside it.
    ///
    /// redb is ACID, so case 3 is a bug, a hardware fault, or a downgrade — not an expected
    /// cost of a crash. The old file is kept as evidence, not discarded as garbage.
    pub fn open(path: &Path) -> Result<Self, anyhow::Error> {
        // `Database::create` is `File::create` underneath and will not create parents. Without
        // this the store works only because something else — the log plugin, on the desktop
        // build — happened to create the app data directory first, which is not a property
        // anyone would know to preserve.
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).with_context(|| {
                format!(
                    "could not create the directory for the player settings store at {}",
                    parent.display()
                )
            })?;
        }

        if Self::is_too_short(path) {
            return Self::start_over(
                path,
                "the file is shorter than a redb header, so it cannot be a database",
            );
        }

        // `Database::create` panics rather than erroring on a database truncated mid-file:
        // `assert!(raw_file_len >= header.layout().len())`. Caught rather than allowed to
        // unwind out of Tauri's setup hook, which would stop the app starting over a file the
        // user does not know exists. Safe to catch here because no `Database` has been
        // constructed yet, so nothing process-global survives the unwind, and the `File` is a
        // plain local whose drop closes the handle and releases the OS lock before the rename
        // below. Two things would break this silently: `panic = "abort"` (not set in this
        // crate or the workspace root), and a future redb version panicking from inside a
        // `Drop` during the unwind, which aborts regardless.
        let created = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            Database::create(path)
        }));

        let created = match created {
            Ok(result) => result,
            Err(_) => {
                return Self::start_over(path, "opening it panicked, so the file is truncated");
            }
        };

        match created {
            Ok(db) => Ok(Self::wrap(db)),
            Err(cause) if Self::is_unreadable(&cause) => {
                Self::start_over(path, &cause.to_string())
            }
            Err(cause) => Err(anyhow::Error::new(cause).context(format!(
                "could not open the player settings store at {}",
                path.display()
            ))),
        }
    }

    /// Whether the file is unreadable as opposed to merely unavailable.
    ///
    /// The distinction decides whether a file is moved aside, so it errs toward "available":
    /// a lock, a permission error or an antivirus scan must never cost a user their settings.
    /// `UnexpectedEof` is the exception among IO errors — a file too short to hold a redb
    /// header is truncated, and no amount of retrying makes it parse.
    fn is_unreadable(cause: &DatabaseError) -> bool {
        match cause {
            // NOT `UpgradeRequired`, and not `RepairAborted`. `UpgradeRequired` means an
            // intact database in an older on-disk format — it fires the first time redb is
            // bumped across a format boundary, which is a routine dependency change, and
            // renaming every user's file aside on that launch would reset everybody's volumes
            // at once. `RepairAborted` covers an unclean shutdown that could not be repaired
            // *this time*, including because the file was opened read-only; an unclean
            // shutdown is expected and recoverable. Both are transient: return `Err`, run in
            // memory, and let a human decide.
            DatabaseError::Storage(StorageError::Corrupted(_)) => true,
            // `InvalidData` is what redb reports for a file whose header is not a redb
            // header, and `UnexpectedEof` for one too short to hold it. Both mean the bytes
            // are not a database, which no amount of retrying changes. Every other IO error
            // — permission, sharing violation, disk full — is a condition that can clear, so
            // it must not cost the user their settings.
            DatabaseError::Storage(StorageError::Io(io)) => {
                matches!(io.kind(), ErrorKind::InvalidData | ErrorKind::UnexpectedEof)
            }
            _ => false,
        }
    }

    /// redb's `DB_HEADER_SIZE` as of 4.1.0 — `TRANSACTION_1_OFFSET + TRANSACTION_SIZE`, i.e.
    /// `(64 + 128) + 128`. It is `pub(super)` over there, so it has to be restated here; if a
    /// future redb grows the header, a stale value silently reopens the band this closes.
    ///
    /// The margin is enormous, which is what makes restating it safe: redb writes the magic
    /// number only *after* resizing the file to its full layout, and the smallest layout is
    /// about a mebibyte. So a file carrying the magic is never anywhere near this short, and
    /// anything that is short is definitively not a database.
    const DB_HEADER_SIZE: u64 = 320;

    /// Whether an existing file is too short to be a database.
    ///
    /// Its own check rather than part of `is_unreadable`, because this input never reaches a
    /// `Result`: on Windows redb reads the header with a `seek_read` loop that treats the
    /// `Ok(0)` at end-of-file as "keep going", so a short file spins a core forever instead of
    /// returning. `catch_unwind` cannot help — an infinite loop is not a panic — and this
    /// guard cannot help with the half-truncated database that panics. The two inputs are
    /// disjoint and both need covering.
    ///
    /// A length of zero is excluded: that is the normal "create me" path. An unreadable
    /// `metadata` is left to `Database::create` and the existing transient classification
    /// rather than guessed at here.
    fn is_too_short(path: &Path) -> bool {
        std::fs::metadata(path)
            .map(|meta| meta.is_file() && meta.len() > 0 && meta.len() < Self::DB_HEADER_SIZE)
            .unwrap_or(false)
    }

    /// Moves an unusable file aside and starts a new database beside it.
    fn start_over(path: &Path, why: &str) -> Result<Self, anyhow::Error> {
        let aside = Self::move_aside(path).with_context(|| {
            format!(
                "player settings at {} are unreadable ({why}) and could not be moved aside",
                path.display()
            )
        })?;
        error!(
            "Player settings at {} could not be read ({why}). The file has been kept at {} and a new one started. Per-player volumes for this device are reset.",
            path.display(),
            aside.display()
        );
        let db = Database::create(path).with_context(|| {
            format!(
                "could not create a replacement player settings store at {}",
                path.display()
            )
        })?;
        Ok(Self::wrap(db))
    }

    fn wrap(db: Database) -> Self {
        Self {
            db,
            skipped: AtomicBool::new(false),
        }
    }

    fn move_aside(path: &Path) -> Result<PathBuf, anyhow::Error> {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|since| since.as_secs())
            .unwrap_or_default();
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("player_settings.redb");
        let aside = path.with_file_name(format!("{name}.corrupt-{stamp}"));
        std::fs::rename(path, &aside)?;
        Ok(aside)
    }
}

impl PlayerSettingsBackend for RedbBackend {
    /// Every row in the file.
    ///
    /// A row whose key or value will not decode is skipped with a warning rather than failing
    /// the load. One unreadable row must not cost the user every other setting they have.
    fn load_all(&self) -> Result<Vec<PlayerSettingsRow>, anyhow::Error> {
        let read = self.db.begin_read()?;
        let table = match read.open_table(TABLE) {
            Ok(table) => table,
            // A database that has never been written has no table yet, which is empty, not
            // an error.
            Err(redb::TableError::TableDoesNotExist(_)) => return Ok(Vec::new()),
            Err(cause) => return Err(cause.into()),
        };

        let mut rows = Vec::new();
        for entry in table.iter()? {
            let (encoded, value) = entry?;
            let Some(key) = PlayerKey::decode(encoded.value()) else {
                warn!(
                    "Skipping a player settings row with an undecodable key: {:?}",
                    encoded.value()
                );
                self.skipped.store(true, Ordering::Relaxed);
                continue;
            };
            match serde_json::from_slice::<PlayerGainSettings>(value.value()) {
                Ok(settings) => rows.push(PlayerSettingsRow::new(key, settings)),
                Err(cause) => {
                    warn!("Skipping player settings for {}: {cause}", key.encode());
                    self.skipped.store(true, Ordering::Relaxed);
                }
            }
        }
        Ok(rows)
    }

    /// Whether the last load could not decode some row.
    ///
    /// The caller uses this to suppress its write-back. Every write is a whole-file rewrite,
    /// so persisting a load that silently dropped rows *deletes* them — and the rows most
    /// likely to be undecodable are the ones written by a newer build, or by one field added
    /// without a `serde` default. Whole-file damage is preserved as evidence; single-row
    /// damage must be too.
    fn skipped_rows(&self) -> bool {
        self.skipped.load(Ordering::Relaxed)
    }

    /// Replaces the file's contents with `rows`, in one transaction.
    ///
    /// A whole rewrite rather than a diff. At this cardinality it costs microseconds, and it
    /// makes the file and the service's in-memory map trivially consistent — there is no
    /// partial-application state for the two to disagree about.
    fn write_all(&self, rows: &[PlayerSettingsRow]) -> Result<(), anyhow::Error> {
        let write = self.db.begin_write()?;
        {
            let mut table = write.open_table(TABLE)?;
            table.retain(|_, _| false)?;
            for row in rows {
                let encoded = serde_json::to_vec(&row.settings)?;
                table.insert(row.key.encode().as_str(), encoded.as_slice())?;
            }
        }
        write.commit()?;
        Ok(())
    }
}
