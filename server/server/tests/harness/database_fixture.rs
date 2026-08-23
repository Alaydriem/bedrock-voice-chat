use anyhow::Result;
use migration::{Migrator, MigratorTrait};
use sea_orm::{Database, DatabaseConnection};
use tempfile::TempDir;

/// A migrated, throwaway SQLite database for service-level tests.
///
/// The directory is held so the file outlives the connection; dropping the fixture removes
/// both.
pub struct DatabaseFixture {
    pub connection: DatabaseConnection,
    _dir: TempDir,
}

impl DatabaseFixture {
    pub async fn create() -> Result<Self> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("test.sqlite");
        let connection =
            Database::connect(format!("sqlite://{}?mode=rwc", path.display())).await?;
        Migrator::up(&connection, None).await?;
        Ok(Self {
            connection,
            _dir: dir,
        })
    }
}
