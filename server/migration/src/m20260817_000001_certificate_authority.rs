use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Deliberately empty of data. The runtime imports whatever is already on disk on its
        // first boot after this lands — see `CaStore::ensure`. Generating a CA here would
        // replace the trust anchor and invalidate every player certificate ever issued.
        manager
            .create_table(
                Table::create()
                    .table(CertificateAuthority::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(CertificateAuthority::Id)
                            .integer()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(CertificateAuthority::CertificatePem)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(CertificateAuthority::KeyPem)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(CertificateAuthority::CreatedAt)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(CertificateAuthority::UpdatedAt)
                            .big_integer()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(CertificateAuthority::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum CertificateAuthority {
    Table,
    Id,
    CertificatePem,
    KeyPem,
    CreatedAt,
    UpdatedAt,
}
