pub mod memory;
pub mod redb;

use common::structs::players::PlayerSettingsRow;

pub use memory::MemoryBackend;
pub use redb::RedbBackend;

/// Where per-player settings are kept.
///
/// Deliberately load-and-store-the-whole-collection rather than a query interface. The service
/// holds the authoritative map in memory and serves every read from it, so a backend never has
/// to answer a question — only to hand over everything at startup and take everything back on a
/// write. That is the narrowest contract that works, and it is what makes the storage engine
/// genuinely replaceable: no keys, no cursors, no transactions in the signature, and nothing
/// engine-specific in the errors.
pub trait PlayerSettingsBackend {
    fn load_all(&self) -> Result<Vec<PlayerSettingsRow>, anyhow::Error>;

    /// Replaces the stored contents with `rows`, atomically.
    ///
    /// Atomicity is the one real demand on an implementation. A partial write would leave the
    /// file disagreeing with the in-memory map for the rest of the session, with nothing to
    /// reconcile them.
    fn write_all(&self, rows: &[PlayerSettingsRow]) -> Result<(), anyhow::Error>;

    /// Whether the last load could not decode some row.
    ///
    /// The service refuses to write back over a partial load, because every write replaces the
    /// whole collection — persisting a load that dropped rows would delete exactly the rows
    /// this build failed to understand.
    fn skipped_rows(&self) -> bool;
}

/// The backend in use, dispatched by match rather than through a trait object.
///
/// Enum delegation is this codebase's convention for exactly this shape (CLAUDE.md §13). It also
/// keeps `PlayerSettingsService` free of a type parameter, which matters because the service is
/// held in Tauri managed state as a concrete `Arc<PlayerSettingsService>`.
pub enum PlayerSettings {
    Redb(RedbBackend),
    Memory(MemoryBackend),
}

impl PlayerSettingsBackend for PlayerSettings {
    fn load_all(&self) -> Result<Vec<PlayerSettingsRow>, anyhow::Error> {
        match self {
            Self::Redb(backend) => backend.load_all(),
            Self::Memory(backend) => backend.load_all(),
        }
    }

    fn write_all(&self, rows: &[PlayerSettingsRow]) -> Result<(), anyhow::Error> {
        match self {
            Self::Redb(backend) => backend.write_all(rows),
            Self::Memory(backend) => backend.write_all(rows),
        }
    }

    fn skipped_rows(&self) -> bool {
        match self {
            Self::Redb(backend) => backend.skipped_rows(),
            Self::Memory(backend) => backend.skipped_rows(),
        }
    }
}
