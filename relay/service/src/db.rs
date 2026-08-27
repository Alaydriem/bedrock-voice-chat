use anyhow::{Context, Result};
use sea_orm::{ConnectOptions, DatabaseConnection};
use sea_orm_migration::MigratorTrait;

use crate::migration::Migrator;

// The registry's database connection.
//
// Migrations run here rather than in a caller: every consumer of this connection
// needs the schema, and a second call to run them is a call somebody eventually
// forgets.
pub struct Db;

impl Db {
    pub async fn connect(url: &str) -> Result<DatabaseConnection> {
        let mut options = ConnectOptions::new(url.to_string());
        options
            .max_connections(32)
            .min_connections(1)
            .sqlx_logging(false);

        let conn = sea_orm::Database::connect(options)
            .await
            .context("connecting to the registry database")?;

        Migrator::up(&conn, None)
            .await
            .context("running registry migrations")?;

        Ok(conn)
    }
}
