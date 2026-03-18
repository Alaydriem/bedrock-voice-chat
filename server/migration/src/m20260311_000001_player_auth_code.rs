use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(PlayerAuthCode::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(PlayerAuthCode::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(PlayerAuthCode::Code)
                            .string()
                            .not_null()
                            .unique_key(),
                    )
                    .col(
                        ColumnDef::new(PlayerAuthCode::PlayerId)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(PlayerAuthCode::ExpiresAt)
                            .big_unsigned()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(PlayerAuthCode::Used)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(PlayerAuthCode::CreatedAt)
                            .big_unsigned()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(PlayerAuthCode::UpdatedAt)
                            .big_unsigned()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_player_auth_code_player_id")
                            .from(PlayerAuthCode::Table, PlayerAuthCode::PlayerId)
                            .to(Player::Table, Player::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(PlayerAuthCode::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum PlayerAuthCode {
    Table,
    Id,
    Code,
    PlayerId,
    ExpiresAt,
    Used,
    CreatedAt,
    UpdatedAt,
}

#[derive(Iden)]
pub enum Player {
    Table,
    Id,
}
