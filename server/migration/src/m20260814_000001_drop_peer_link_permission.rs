use sea_orm_migration::prelude::*;

use crate::m20260322_000002_player_permission::PlayerPermission;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .exec_stmt(
                Query::delete()
                    .from_table(PlayerPermission::Table)
                    .and_where(Expr::col(PlayerPermission::Permission).eq("peer_link"))
                    .to_owned(),
            )
            .await
    }

    // Irreversible by nature: which players held the grant is not recorded
    // anywhere else, so there is nothing to restore.
    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}
