use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(GameAccessToken::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(GameAccessToken::Id)
                            .text()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(GameAccessToken::SecretHash)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(GameAccessToken::CreatedAt)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(GameAccessToken::RevokedAt)
                            .big_integer()
                            .null(),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(GameAccessToken::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum GameAccessToken {
    Table,
    Id,
    SecretHash,
    CreatedAt,
    RevokedAt,
}
