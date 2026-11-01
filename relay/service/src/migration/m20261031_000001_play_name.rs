use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum PlayName {
    Table,
    NodeId,
    Name,
    ClaimedFrom,
    DeclaredAddress,
    DeclaredPort,
    PublishedAddress,
    VerifiedAt,
    CheckinNonce,
    CheckedInAt,
    CreatedAt,
    ReleasedAt,
}

#[derive(DeriveIden)]
enum PlayIssuance {
    Table,
    Id,
    NodeId,
    IssuedAt,
}

#[derive(DeriveIden)]
enum PlayZoneState {
    Table,
    Id,
    Serial,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(PlayName::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(PlayName::NodeId)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(PlayName::Name).string().not_null().unique_key())
                    .col(ColumnDef::new(PlayName::ClaimedFrom).string().not_null())
                    .col(ColumnDef::new(PlayName::DeclaredAddress).string().null())
                    .col(ColumnDef::new(PlayName::DeclaredPort).integer().null())
                    .col(ColumnDef::new(PlayName::PublishedAddress).string().null())
                    .col(ColumnDef::new(PlayName::VerifiedAt).big_integer().null())
                    .col(ColumnDef::new(PlayName::CheckinNonce).string().null())
                    .col(ColumnDef::new(PlayName::CheckedInAt).big_integer().null())
                    .col(ColumnDef::new(PlayName::CreatedAt).big_integer().not_null())
                    .col(ColumnDef::new(PlayName::ReleasedAt).big_integer().null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_play_name_claimed_from_created_at")
                    .table(PlayName::Table)
                    .col(PlayName::ClaimedFrom)
                    .col(PlayName::CreatedAt)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(PlayZoneState::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(PlayZoneState::Id)
                            .integer()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(PlayZoneState::Serial).big_integer().not_null())
                    .to_owned(),
            )
            .await?;

        manager
            .get_connection()
            .execute_unprepared("INSERT INTO play_zone_state (id, serial) VALUES (1, 0)")
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(PlayIssuance::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(PlayIssuance::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(PlayIssuance::NodeId).string().not_null())
                    .col(ColumnDef::new(PlayIssuance::IssuedAt).big_integer().not_null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_play_issuance_node_id_issued_at")
                    .table(PlayIssuance::Table)
                    .col(PlayIssuance::NodeId)
                    .col(PlayIssuance::IssuedAt)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(PlayIssuance::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(PlayZoneState::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(PlayName::Table).to_owned())
            .await
    }
}
