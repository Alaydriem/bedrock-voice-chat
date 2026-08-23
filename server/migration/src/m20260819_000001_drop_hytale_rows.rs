use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let conn = manager.get_connection();

        // `player.game` and `player_identity.game` are enum-backed columns. Once the enum
        // stops carrying the value, a surviving row fails to deserialize at query time rather
        // than at boot, so the error surfaces inside whatever unrelated query happens to read
        // it. Deleting the rows is what keeps that from happening.
        //
        // Children first, and by parent reference as well as by their own column: a child row
        // does not have to agree with its parent about the game, and deleting the parent alone
        // would leave it pointing at an id that no longer exists.
        for sql in [
            "DELETE FROM audio_file \
             WHERE game = 'hytale' \
                OR uploader_id IN (SELECT id FROM player WHERE game = 'hytale')",
            "DELETE FROM player_identity \
             WHERE game = 'hytale' \
                OR player_id IN (SELECT id FROM player WHERE game = 'hytale')",
            "DELETE FROM player WHERE game = 'hytale'",
        ] {
            conn.execute_unprepared(sql).await?;
        }

        Ok(())
    }

    // The rows are not recoverable, and the enum variant that gave them meaning no longer
    // exists, so there is nothing to restore them to.
    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}
