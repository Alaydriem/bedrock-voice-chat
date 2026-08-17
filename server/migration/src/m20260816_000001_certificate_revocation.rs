use common::structs::certificate::{CertificateFingerprint, CertificateValidity};
use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::ConnectionTrait;

use crate::m20231220_000001_player::Player;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(CertificateRevocation::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(CertificateRevocation::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(CertificateRevocation::Fingerprint)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(CertificateRevocation::PlayerId)
                            .integer()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(CertificateRevocation::Reason)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(CertificateRevocation::RevokedAt)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(CertificateRevocation::ExpiresAt)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(CertificateRevocation::CreatedAt)
                            .big_integer()
                            .not_null(),
                    )
                    // Set null rather than cascade: a revocation has to outlive the identity
                    // it revoked, or deleting a player silently un-revokes their certificate.
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_certificate_revocation_player")
                            .from(
                                CertificateRevocation::Table,
                                CertificateRevocation::PlayerId,
                            )
                            .to(Player::Table, Player::Id)
                            .on_delete(ForeignKeyAction::SetNull),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_certificate_revocation_fingerprint")
                    .table(CertificateRevocation::Table)
                    .col(CertificateRevocation::Fingerprint)
                    .unique()
                    .to_owned(),
            )
            .await?;

        Self::revoke_existing_banished(manager).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(CertificateRevocation::Table).to_owned())
            .await
    }
}

impl Migration {
    // Everyone banned before this migration ran still holds a working certificate, which is
    // the exact population the revocation list exists for. Without this the feature ships and
    // changes nothing for anybody already banned.
    async fn revoke_existing_banished(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
        let connection = manager.get_connection();

        let banished = Query::select()
            .column(Player::Id)
            .column(Player::Certificate)
            .from(Player::Table)
            .and_where(Expr::col(Player::Banished).eq(true))
            .to_owned();

        let rows = connection.query_all(&banished).await?;
        let now = common::ncryptflib::rocket::Utc::now().timestamp();

        for row in rows {
            let player_id: i32 = row.try_get("", "id")?;
            let pem: String = row.try_get("", "certificate")?;

            let Some(fingerprint) = CertificateFingerprint::from_pem(&pem) else {
                tracing::warn!(
                    player_id,
                    "certificate revocation backfill: stored certificate did not parse, \
                     so there is nothing to revoke for this player"
                );
                continue;
            };

            // A certificate whose validity cannot be read is treated as already expired. The
            // row still blocks the certificate; only the pruner cares about this value.
            let expires_at = CertificateValidity::not_after(&pem).unwrap_or(now);

            let insert = Query::insert()
                .into_table(CertificateRevocation::Table)
                .columns([
                    CertificateRevocation::Fingerprint,
                    CertificateRevocation::PlayerId,
                    CertificateRevocation::Reason,
                    CertificateRevocation::RevokedAt,
                    CertificateRevocation::ExpiresAt,
                    CertificateRevocation::CreatedAt,
                ])
                .values_panic([
                    fingerprint.into(),
                    player_id.into(),
                    "banished before certificate revocation existed".into(),
                    now.into(),
                    expires_at.into(),
                    now.into(),
                ])
                .to_owned();

            connection.execute(&insert).await?;
        }

        Ok(())
    }
}

#[derive(Iden)]
pub enum CertificateRevocation {
    Table,
    Id,
    Fingerprint,
    PlayerId,
    Reason,
    RevokedAt,
    ExpiresAt,
    CreatedAt,
}
