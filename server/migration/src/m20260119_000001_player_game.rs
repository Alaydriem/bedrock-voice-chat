use sea_orm::Statement;
use sea_orm_migration::{self, prelude::*, sea_orm::ConnectionTrait};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        // Add game column with default 'minecraft'
        db.execute(Statement::from_string(
            manager.get_database_backend(),
            "ALTER TABLE player ADD COLUMN game TEXT NOT NULL DEFAULT 'minecraft'".to_string(),
        ))
        .await?;

        // Create index for gamertag + game lookups
        db.execute(Statement::from_string(
            manager.get_database_backend(),
            "CREATE INDEX idx_player_gamertag_game ON player(gamertag, game)".to_string(),
        ))
        .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        // Drop the index first
        db.execute(Statement::from_string(
            manager.get_database_backend(),
            "DROP INDEX IF EXISTS idx_player_gamertag_game".to_string(),
        ))
        .await?;

        // SQLite doesn't support DROP COLUMN directly, so we skip it for down migration
        // In production, this would require a table rebuild for SQLite

        Ok(())
    }
}