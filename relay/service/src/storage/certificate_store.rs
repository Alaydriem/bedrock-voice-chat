use std::sync::Arc;

use sea_orm::{ActiveValue, DatabaseConnection, EntityTrait, sea_query::OnConflict};

use crate::entity::certificate;

use super::error::StorageError;
use super::material::CertificateMaterial;

// The registry's own certificate, held in the database.
//
// Never written to disk. `RustlsConfig::from_pem` takes the material directly, so there
// is no cert directory, no volume to mount, and no second copy that can disagree with
// this one.
//
// Keyed by hostname, so changing `http.hostname` cannot serve the previous name's
// certificate: the lookup simply misses and an issuance is owed.
pub struct CertificateStore {
    conn: Arc<DatabaseConnection>,
}

impl CertificateStore {
    pub fn new(conn: Arc<DatabaseConnection>) -> Self {
        Self { conn }
    }

    pub fn new_shared(conn: Arc<DatabaseConnection>) -> Arc<Self> {
        Arc::new(Self::new(conn))
    }

    pub async fn read(&self, hostname: &str) -> Result<Option<CertificateMaterial>, StorageError> {
        Ok(certificate::Entity::find_by_id(hostname)
            .one(self.conn.as_ref())
            .await?
            .map(|row| CertificateMaterial::new(row.chain_pem, row.key_pem)))
    }

    // Upsert rather than insert. A renewal replaces the row for the same hostname, and
    // an insert would fail on the second issuance — sixty days after anyone last
    // watched it happen.
    pub async fn write(
        &self,
        hostname: &str,
        material: &CertificateMaterial,
    ) -> Result<(), StorageError> {
        certificate::Entity::insert(certificate::ActiveModel {
            hostname: ActiveValue::Set(hostname.to_string()),
            chain_pem: ActiveValue::Set(material.chain_pem.clone()),
            key_pem: ActiveValue::Set(material.key_pem.clone()),
            issued_at: ActiveValue::Set(Self::now()),
        })
        .on_conflict(
            OnConflict::column(certificate::Column::Hostname)
                .update_columns([
                    certificate::Column::ChainPem,
                    certificate::Column::KeyPem,
                    certificate::Column::IssuedAt,
                ])
                .to_owned(),
        )
        .exec(self.conn.as_ref())
        .await?;

        Ok(())
    }

    fn now() -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or_default()
    }
}
