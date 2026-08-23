use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ServerSecret::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ServerSecret::Name)
                            .text()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(ServerSecret::Value).text().not_null())
                    .col(
                        ColumnDef::new(ServerSecret::CreatedAt)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ServerSecret::UpdatedAt)
                            .big_integer()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        // The certificate and its key share a row so they cannot diverge. `directory_url`
        // and `names` travel with them: a change to either invalidates the stored
        // certificate instead of serving one issued for the wrong provider or domains.
        manager
            .create_table(
                Table::create()
                    .table(AcmeCredential::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(AcmeCredential::Id)
                            .integer()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(AcmeCredential::AccountJson)
                            .text()
                            .not_null(),
                    )
                    .col(ColumnDef::new(AcmeCredential::CertificatePem).text().null())
                    .col(ColumnDef::new(AcmeCredential::KeyPem).text().null())
                    .col(
                        ColumnDef::new(AcmeCredential::DirectoryUrl)
                            .text()
                            .not_null(),
                    )
                    .col(ColumnDef::new(AcmeCredential::Names).text().not_null())
                    .col(
                        ColumnDef::new(AcmeCredential::CreatedAt)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(AcmeCredential::UpdatedAt)
                            .big_integer()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(AcmeCredential::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(ServerSecret::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum ServerSecret {
    Table,
    Name,
    Value,
    CreatedAt,
    UpdatedAt,
}

#[derive(Iden)]
pub enum AcmeCredential {
    Table,
    Id,
    AccountJson,
    CertificatePem,
    KeyPem,
    DirectoryUrl,
    Names,
    CreatedAt,
    UpdatedAt,
}
