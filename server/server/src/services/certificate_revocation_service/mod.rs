use std::sync::Arc;
use std::time::Duration;

use anyhow::anyhow;
use common::structs::certificate::{CertificateFingerprint, CertificateValidity};
use entity::certificate_revocation;
use moka::future::Cache;
use sea_orm::{
    ActiveValue, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, sea_query::OnConflict,
};

/// Answers whether a presented certificate has been revoked, and records revocations.
///
/// The cache holds both answers. A negative is by far the common case and is what keeps this
/// off the database on every request; `revoke` invalidates it so a ban does not wait out a TTL.
pub struct CertificateRevocationService {
    cache: Cache<String, bool>,
}

impl CertificateRevocationService {
    const CACHE_TTL: Duration = Duration::from_secs(300);
    const CACHE_CAPACITY: u64 = 4096;

    pub fn new() -> Self {
        Self {
            cache: Cache::builder()
                .time_to_live(Self::CACHE_TTL)
                .max_capacity(Self::CACHE_CAPACITY)
                .build(),
        }
    }

    pub fn new_shared() -> Arc<Self> {
        Arc::new(Self::new())
    }

    /// An empty fingerprint is never revoked and never matches anything.
    ///
    /// A stored certificate that does not parse produces no fingerprint, so treating empty as
    /// a value would let it collide with whatever the database returned first.
    pub async fn is_revoked<C: ConnectionTrait>(&self, conn: &C, fingerprint: &str) -> bool {
        if fingerprint.is_empty() {
            return false;
        }

        if let Some(cached) = self.cache.get(fingerprint).await {
            return cached;
        }

        let revoked = match certificate_revocation::Entity::find()
            .filter(certificate_revocation::Column::Fingerprint.eq(fingerprint))
            .one(conn)
            .await
        {
            Ok(found) => found.is_some(),
            Err(e) => {
                // Fail closed, and do not cache. A database that cannot answer must not be
                // read as "not revoked", and must not have that reading remembered.
                tracing::error!("certificate revocation lookup failed: {}", e);
                return true;
            }
        };

        self.cache.insert(fingerprint.to_string(), revoked).await;
        revoked
    }

    pub async fn revoke<C: ConnectionTrait>(
        &self,
        conn: &C,
        fingerprint: &str,
        player_id: Option<i32>,
        reason: &str,
        expires_at: i64,
    ) -> Result<(), anyhow::Error> {
        if fingerprint.is_empty() {
            return Err(anyhow!("refusing to revoke an empty fingerprint"));
        }

        let now = common::ncryptflib::rocket::Utc::now().timestamp();
        let record = certificate_revocation::ActiveModel {
            id: ActiveValue::NotSet,
            fingerprint: ActiveValue::Set(fingerprint.to_string()),
            player_id: ActiveValue::Set(player_id),
            reason: ActiveValue::Set(reason.to_string()),
            revoked_at: ActiveValue::Set(now),
            expires_at: ActiveValue::Set(expires_at),
            created_at: ActiveValue::Set(now),
        };

        // A second ban of the same certificate is a no-op, not an error.
        //
        // The conflict target is named explicitly: the collision is on the unique index over
        // `fingerprint`, not on the primary key, so an untargeted DO NOTHING does not cover it.
        // `try_insert` is what makes an insert that changed nothing a success rather than a
        // "no records inserted" error.
        certificate_revocation::Entity::insert(record)
            .on_conflict(
                OnConflict::column(certificate_revocation::Column::Fingerprint)
                    .do_nothing()
                    .to_owned(),
            )
            .try_insert()
            .exec(conn)
            .await?;

        self.cache.invalidate(fingerprint).await;
        Ok(())
    }

    /// Revokes whatever certificate a stored PEM holds.
    ///
    /// `expires_at` comes from the certificate's own `notAfter`, so a pruner can drop the row
    /// once the certificate could not be presented anyway.
    pub async fn revoke_pem<C: ConnectionTrait>(
        &self,
        conn: &C,
        pem: &str,
        player_id: Option<i32>,
        reason: &str,
    ) -> Result<(), anyhow::Error> {
        let fingerprint = CertificateFingerprint::from_pem(pem)
            .ok_or_else(|| anyhow!("stored certificate did not parse; nothing to revoke"))?;

        // A certificate whose validity cannot be read is treated as already expired. The row
        // still blocks the certificate; only the pruner reads this value.
        let expires_at = CertificateValidity::not_after(pem)
            .unwrap_or_else(|| common::ncryptflib::rocket::Utc::now().timestamp());

        self.revoke(conn, &fingerprint, player_id, reason, expires_at)
            .await
    }
}

impl Default for CertificateRevocationService {
    fn default() -> Self {
        Self::new()
    }
}
