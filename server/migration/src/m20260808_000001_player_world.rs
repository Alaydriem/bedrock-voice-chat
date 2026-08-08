use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(PlayerWorld::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(PlayerWorld::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(PlayerWorld::PlayerId).integer().not_null())
                    // Polymorphic by design: a random UUID minted by the mod, or a derived
                    // hash on the proxy path. Sized for both and never parsed.
                    .col(
                        ColumnDef::new(PlayerWorld::WorldUuid)
                            .string_len(128)
                            .not_null(),
                    )
                    .col(ColumnDef::new(PlayerWorld::WorldName).string().not_null())
                    // Epoch seconds as signed, matching m20260726_000001_signed_timestamps.
                    // Unsigned stopped decoding on MySQL once the entities moved to i64.
                    .col(
                        ColumnDef::new(PlayerWorld::LastSeen)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(PlayerWorld::CreatedAt)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(PlayerWorld::UpdatedAt)
                            .big_integer()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_player_world_player_id")
                            .from(PlayerWorld::Table, PlayerWorld::PlayerId)
                            .to(Player::Table, Player::Id),
                    )
                    .to_owned(),
            )
            .await?;

        // The upsert conflicts on this pair, so it has to be unique rather than merely
        // indexed. A player is seen in a world constantly; without it the table grows forever.
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_player_world_player_world")
                    .table(PlayerWorld::Table)
                    .col(PlayerWorld::PlayerId)
                    .col(PlayerWorld::WorldUuid)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_player_world_world_uuid")
                    .table(PlayerWorld::Table)
                    .col(PlayerWorld::WorldUuid)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(PlayerWorld::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum PlayerWorld {
    Table,
    Id,
    PlayerId,
    WorldUuid,
    WorldName,
    LastSeen,
    CreatedAt,
    UpdatedAt,
}

#[derive(Iden)]
pub enum Player {
    Table,
    Id,
}
