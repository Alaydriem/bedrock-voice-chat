use sea_orm_migration::prelude::*;
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        match manager
            .create_table(
                Table::create()
                    .table(Player::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Player::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Player::Gamertag).string())
                    .col(ColumnDef::new(Player::Gamerpic).string())
                    .col(ColumnDef::new(Player::Certificate).string())
                    .col(ColumnDef::new(Player::CertificateKey).string())
                    .col(ColumnDef::new(Player::Keypair).binary())
                    .col(ColumnDef::new(Player::Signature).binary())
                    .col(ColumnDef::new(Player::Banished).boolean())
                    .col(ColumnDef::new(Player::CreatedAt).big_unsigned().not_null())
                    .col(ColumnDef::new(Player::UpdatedAt).big_unsigned().not_null())
                    .to_owned(),
            )
            .await
        {
            Ok(_result) => Ok(()),
            Err(_) => Err(DbErr::Migration(
                "Unable to migrate `Player` table.".to_owned(),
            )),
        }
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        return manager
            .drop_table(Table::drop().table(Player::Table).to_owned())
            .await;
    }
}

#[derive(Iden)]
pub enum Player {
    Table,
    Id,
    Gamertag,
    Gamerpic,
    Banished,
    Keypair,
    Certificate,
    CertificateKey,
    Signature,
    CreatedAt,
    UpdatedAt,
}
