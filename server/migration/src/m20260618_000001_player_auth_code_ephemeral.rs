use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(PlayerAuthCode::Table)
                    .add_column(
                        ColumnDef::new(PlayerAuthCode::Ephemeral)
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(PlayerAuthCode::Table)
                    .drop_column(PlayerAuthCode::Ephemeral)
                    .to_owned(),
            )
            .await
    }
}

#[derive(Iden)]
pub enum PlayerAuthCode {
    Table,
    Ephemeral,
}
