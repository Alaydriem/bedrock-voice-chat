use anyhow::Result;
use migration::{Migrator, MigratorTrait};
use sea_orm::{ConnectOptions, Database, DatabaseConnection};
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
    const PAUSED_CLOCK_CONNECTIONS: u32 = 4;

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

    /// A database whose pool owns no timers once it is connected.
    ///
    /// A test that pauses the tokio clock jumps it to the next timer whenever the runtime is
    /// idle, and waiting on the database's worker thread counts as idle. Pool timeouts and
    /// the idle reaper would then fire at once and fail the query. One connection, opened
    /// before the clock is paused and never recycled, leaves nothing for the clock to jump
    /// to.
    ///
    /// Limit: four connections. A test that holds more than four checkouts at once would wait
    /// for a fifth with the acquire timeout running, and the paused clock would then jump to
    /// that timeout and fail the query.
    pub async fn create_for_paused_clock() -> Result<Self> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("test.sqlite");

        let mut options = ConnectOptions::new(format!("sqlite://{}?mode=rwc", path.display()));
        options
            .max_connections(Self::PAUSED_CLOCK_CONNECTIONS)
            .min_connections(1)
            .idle_timeout(None)
            .max_lifetime(None)
            // The ping on checkout waits on the worker thread with the acquire timeout
            // running, which is the same idle-with-a-timer state.
            .test_before_acquire(false)
            .sqlx_logging(false);
        let connection = Database::connect(options).await?;
        Migrator::up(&connection, None).await?;

        // A connection goes back to the pool on a task of its own, so the next checkout
        // can find the pool empty and wait with the acquire timeout running. Opening every
        // connection now, at once, leaves an idle one for each checkout.
        futures_util::future::try_join_all(
            (0..Self::PAUSED_CLOCK_CONNECTIONS).map(|_| connection.ping()),
        )
        .await?;

        Ok(Self {
            connection,
            _dir: dir,
        })
    }
}
