use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(DeriveIden)]
enum NodeKey {
    Table,
    Name,
    Value,
    CreatedAt,
}

#[derive(DeriveIden)]
enum Certificate {
    Table,
    Hostname,
    ChainPem,
    KeyPem,
    IssuedAt,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(NodeKey::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(NodeKey::Name).string().not_null().primary_key())
                    .col(ColumnDef::new(NodeKey::Value).string().not_null())
                    .col(ColumnDef::new(NodeKey::CreatedAt).big_integer().not_null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Certificate::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Certificate::Hostname)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Certificate::ChainPem).text().not_null())
                    .col(ColumnDef::new(Certificate::KeyPem).text().not_null())
                    .col(ColumnDef::new(Certificate::IssuedAt).big_integer().not_null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Certificate::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(NodeKey::Table).to_owned())
            .await
    }
}
