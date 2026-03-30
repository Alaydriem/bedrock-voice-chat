use sea_orm_migration::prelude::*;

use crate::m20231220_000001_player::Player;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(PlayerPermission::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(PlayerPermission::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(PlayerPermission::PlayerId).integer().not_null())
                    .col(ColumnDef::new(PlayerPermission::Permission).string().not_null())
                    .col(ColumnDef::new(PlayerPermission::Effect).integer().not_null())
                    .col(ColumnDef::new(PlayerPermission::CreatedAt).big_integer().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_player_permission_player")
                            .from(PlayerPermission::Table, PlayerPermission::PlayerId)
                            .to(Player::Table, Player::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_player_permission_unique")
                    .table(PlayerPermission::Table)
                    .col(PlayerPermission::PlayerId)
                    .col(PlayerPermission::Permission)
                    .unique()
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(PlayerPermission::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum PlayerPermission {
    Table,
    Id,
    PlayerId,
    Permission,
    Effect,
    CreatedAt,
}
