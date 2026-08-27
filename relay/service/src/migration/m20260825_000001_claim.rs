use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum Claim {
    Table,
    Id,
    Token,
    ExpiresAt,
    ConsumedAt,
    CreatedAt,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Claim::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Claim::Id).string().not_null().primary_key())
                    .col(ColumnDef::new(Claim::Token).string().not_null())
                    .col(ColumnDef::new(Claim::ExpiresAt).big_integer().not_null())
                    .col(ColumnDef::new(Claim::ConsumedAt).big_integer().null())
                    .col(ColumnDef::new(Claim::CreatedAt).big_integer().not_null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Claim::Table).to_owned())
            .await
    }
}
