use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // The digest is the key: the plaintext is never stored, so there is no other
        // column a redemption could look a row up by.
        manager
            .create_table(
                Table::create()
                    .table(PeerPairingCode::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(PeerPairingCode::CodeDigest)
                            .text()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(PeerPairingCode::Label).text().not_null())
                    .col(
                        ColumnDef::new(PeerPairingCode::ExpiresAt)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(PeerPairingCode::ConsumedAt)
                            .big_integer()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(PeerPairingCode::Attempts)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(PeerPairingCode::CreatedAt)
                            .big_integer()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        // Keyed by the node's public key, because that is what the peer's TLS certificate
        // proves and therefore the only identity a grant can be pinned to.
        manager
            .create_table(
                Table::create()
                    .table(PeerGrant::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(PeerGrant::NodeId)
                            .text()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(PeerGrant::Label).text().not_null())
                    .col(ColumnDef::new(PeerGrant::Worlds).text().not_null())
                    .col(ColumnDef::new(PeerGrant::Capabilities).text().not_null())
                    .col(ColumnDef::new(PeerGrant::PairedAt).big_integer().not_null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(PeerGrant::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(PeerPairingCode::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum PeerPairingCode {
    Table,
    CodeDigest,
    Label,
    ExpiresAt,
    ConsumedAt,
    Attempts,
    CreatedAt,
}

#[derive(Iden)]
pub enum PeerGrant {
    Table,
    NodeId,
    Label,
    Worlds,
    Capabilities,
    PairedAt,
}
