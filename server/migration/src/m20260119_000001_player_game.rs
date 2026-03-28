use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Player::Table)
                    .add_column(
                        ColumnDef::new(Player::Game)
                            .string()
                            .not_null()
                            .default("minecraft"),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_player_gamertag_game")
                    .table(Player::Table)
                    .col(Player::Gamertag)
                    .col(Player::Game)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(
                Index::drop()
                    .name("idx_player_gamertag_game")
                    .table(Player::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Player::Table)
                    .drop_column(Player::Game)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(Iden)]
pub enum Player {
    Table,
    Gamertag,
    Game,
}
