use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::DatabaseBackend;

#[derive(DeriveMigrationName)]
pub struct Migration;

// Epoch-second columns were created as BIGINT UNSIGNED (player, player_auth_code)
// or INT (player_identity) before Postgres support. The entities now use i64,
// which sqlx only decodes from signed BIGINT on MySQL, so existing MySQL
// installs are altered here. SQLite integers are dynamically sized 64-bit and
// Postgres installs are created signed from the start, so both are no-ops.
#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DatabaseBackend::MySql {
            return Ok(());
        }

        manager
            .alter_table(
                Table::alter()
                    .table(Player::Table)
                    .modify_column(ColumnDef::new(Player::CreatedAt).big_integer().not_null())
                    .modify_column(ColumnDef::new(Player::UpdatedAt).big_integer().not_null())
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(PlayerAuthCode::Table)
                    .modify_column(
                        ColumnDef::new(PlayerAuthCode::ExpiresAt)
                            .big_integer()
                            .not_null(),
                    )
                    .modify_column(
                        ColumnDef::new(PlayerAuthCode::CreatedAt)
                            .big_integer()
                            .not_null(),
                    )
                    .modify_column(
                        ColumnDef::new(PlayerAuthCode::UpdatedAt)
                            .big_integer()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(PlayerIdentity::Table)
                    .modify_column(
                        ColumnDef::new(PlayerIdentity::CreatedAt)
                            .big_integer()
                            .not_null(),
                    )
                    .modify_column(
                        ColumnDef::new(PlayerIdentity::UpdatedAt)
                            .big_integer()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DatabaseBackend::MySql {
            return Ok(());
        }

        manager
            .alter_table(
                Table::alter()
                    .table(Player::Table)
                    .modify_column(ColumnDef::new(Player::CreatedAt).big_unsigned().not_null())
                    .modify_column(ColumnDef::new(Player::UpdatedAt).big_unsigned().not_null())
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(PlayerAuthCode::Table)
                    .modify_column(
                        ColumnDef::new(PlayerAuthCode::ExpiresAt)
                            .big_unsigned()
                            .not_null(),
                    )
                    .modify_column(
                        ColumnDef::new(PlayerAuthCode::CreatedAt)
                            .big_unsigned()
                            .not_null(),
                    )
                    .modify_column(
                        ColumnDef::new(PlayerAuthCode::UpdatedAt)
                            .big_unsigned()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(PlayerIdentity::Table)
                    .modify_column(
                        ColumnDef::new(PlayerIdentity::CreatedAt)
                            .integer()
                            .not_null(),
                    )
                    .modify_column(
                        ColumnDef::new(PlayerIdentity::UpdatedAt)
                            .integer()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await
    }
}

#[derive(Iden)]
enum Player {
    Table,
    CreatedAt,
    UpdatedAt,
}

#[derive(Iden)]
enum PlayerAuthCode {
    Table,
    ExpiresAt,
    CreatedAt,
    UpdatedAt,
}

#[derive(Iden)]
enum PlayerIdentity {
    Table,
    CreatedAt,
    UpdatedAt,
}
