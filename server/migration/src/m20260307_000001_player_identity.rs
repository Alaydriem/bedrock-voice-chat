use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(PlayerIdentity::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(PlayerIdentity::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(PlayerIdentity::PlayerId)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(PlayerIdentity::Alias)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(PlayerIdentity::Game)
                            .string()
                            .not_null()
                            .default("minecraft"),
                    )
                    .col(
                        ColumnDef::new(PlayerIdentity::AliasType)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(PlayerIdentity::CreatedAt)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(PlayerIdentity::UpdatedAt)
                            .integer()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_player_identity_player_id")
                            .from(PlayerIdentity::Table, PlayerIdentity::PlayerId)
                            .to(Player::Table, Player::Id),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_player_identity_alias_game")
                    .table(PlayerIdentity::Table)
                    .col(PlayerIdentity::Alias)
                    .col(PlayerIdentity::Game)
                    .unique()
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(PlayerIdentity::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum PlayerIdentity {
    Table,
    Id,
    PlayerId,
    Alias,
    Game,
    AliasType,
    CreatedAt,
    UpdatedAt,
}

#[derive(Iden)]
pub enum Player {
    Table,
    Id,
}
