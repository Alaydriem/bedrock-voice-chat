//! Database-backed storage for the deployment's certificate authority.
//!
//! The database is the source of truth. The bytes are materialised to `certs_path` on every
//! boot because the TLS stacks take file paths and read them once at ignite — Rocket's
//! `tls.mutual.ca_certs`, the WebSocket listener's trust root, and `CertificateService`, which
//! loads the root to sign player leaves. Nothing in the process reads the CA from anywhere but
//! disk; what changed is where the durable copy lives.
//!
//! The point is that `certs_path` can now be a temp directory. A container needs no persistent
//! volume.

use common::curia;
use anyhow::{Context, Result, anyhow};
use entity::certificate_authority;
use sea_orm::{ActiveValue, ConnectionTrait, EntityTrait, PaginatorTrait};
use std::fs;
use std::path::Path;

use super::ca_cert::CaCertManager;

/// The single row's primary key. There is exactly one CA per deployment.
const CA_ROW_ID: i32 = 1;

pub struct CaStore;

impl CaStore {
    /// Whether the database already holds a certificate authority.
    pub async fn exists<C: ConnectionTrait>(conn: &C) -> Result<bool> {
        Ok(certificate_authority::Entity::find().count(conn).await? > 0)
    }

    /// Resolves the deployment's CA, leaving it both in the database and on disk.
    ///
    /// Order matters, and the middle case is the one that protects existing deployments:
    ///
    /// 1. **In the database** — written to `certs_path` so the TLS stacks can read it.
    /// 2. **Not in the database but on disk** — imported. An upgrade must adopt the CA the
    ///    deployment already has; minting a fresh one would invalidate every player
    ///    certificate ever issued by it.
    /// 3. **Neither** — generated, then stored.
    ///
    /// After that, `CaCertManager::ensure` runs against the materialised files and re-signs
    /// the certificate if the configured SAN set has drifted. A re-sign keeps the same
    /// keypair, so the trust anchor is unchanged; the new certificate bytes are written back
    /// to the database so the next boot does not repeat the work.
    pub async fn ensure<C: ConnectionTrait>(
        conn: &C,
        certs_path: &str,
        san_strings: &[String],
    ) -> Result<(String, String)> {
        let dir = Path::new(certs_path);
        fs::create_dir_all(dir)
            .with_context(|| format!("creating certs dir {}", dir.display()))?;

        let stored = certificate_authority::Entity::find_by_id(CA_ROW_ID)
            .one(conn)
            .await?;

        if let Some(row) = &stored {
            Self::materialise(dir, &row.certificate_pem, &row.key_pem)?;
        } else if dir.join("ca.key").exists() {
            curia::info!(
                "Importing the existing on-disk certificate authority into the database. \
                 The trust anchor is unchanged, so every certificate it has issued stays valid."
            );
        }

        // Re-signs on SAN drift, generates when nothing is on disk, and never replaces an
        // existing keypair. All of that behaviour is unchanged and already covered by its
        // own tests.
        let (cert_pem, key_pem) = CaCertManager::new(certs_path).ensure(san_strings)?;

        let unchanged = stored
            .as_ref()
            .is_some_and(|row| row.certificate_pem == cert_pem && row.key_pem == key_pem);
        if !unchanged {
            Self::persist(conn, stored.is_some(), &cert_pem, &key_pem).await?;
        }

        Ok((cert_pem, key_pem))
    }

    /// Writes the CA to `certs_path` for the TLS stacks to read.
    ///
    /// A file whose contents already match is left alone, so a restart does not churn the
    /// mtime of material nothing has changed. A file that disagrees is renamed rather than
    /// overwritten: the database wins, but the bytes it displaced stay recoverable.
    fn materialise(dir: &Path, certificate_pem: &str, key_pem: &str) -> Result<()> {
        let now = common::ncryptflib::rocket::Utc::now().timestamp();

        for (name, contents) in [("ca.crt", certificate_pem), ("ca.key", key_pem)] {
            let path = dir.join(name);
            match fs::read_to_string(&path) {
                Ok(existing) if existing == contents => continue,
                Ok(_) => {
                    let superseded = dir.join(format!("{name}.superseded-{now}"));
                    fs::rename(&path, &superseded).with_context(|| {
                        format!("preserving {} as {}", path.display(), superseded.display())
                    })?;
                    curia::warn!("The stored certificate authority differs from the one on disk. The \
                         database is authoritative; the displaced file is preserved.", { "path": path.display().to_string(), "preserved_as": superseded.display().to_string() });
                }
                Err(_) => {}
            }
            fs::write(&path, contents)
                .with_context(|| format!("writing {} at {}", name, path.display()))?;
        }
        Ok(())
    }

    async fn persist<C: ConnectionTrait>(
        conn: &C,
        exists: bool,
        certificate_pem: &str,
        key_pem: &str,
    ) -> Result<()> {
        // The pair `CaCertManager::ensure` returns always corresponds, but the guard belongs
        // next to the write as well: this is where a mismatch would become permanent, and a
        // caller two modules away is not where that invariant should live.
        let keypair = rcgen::KeyPair::from_pem(key_pem)
            .map_err(|e| anyhow!("parsing the certificate authority key: {e}"))?;
        if !super::ca_cert::KeyMatch::matches(certificate_pem, &keypair) {
            return Err(anyhow!(
                "refusing to store a certificate authority whose certificate was not issued \
                 by its key"
            ));
        }

        let now = common::ncryptflib::rocket::Utc::now().timestamp();

        if exists {
            let model = certificate_authority::ActiveModel {
                id: ActiveValue::Unchanged(CA_ROW_ID),
                certificate_pem: ActiveValue::Set(certificate_pem.to_string()),
                key_pem: ActiveValue::Set(key_pem.to_string()),
                created_at: ActiveValue::NotSet,
                updated_at: ActiveValue::Set(now),
            };
            certificate_authority::Entity::update(model)
                .exec(conn)
                .await
                .map_err(|e| anyhow!("updating the stored certificate authority: {e}"))?;
            return Ok(());
        }

        let model = certificate_authority::ActiveModel {
            id: ActiveValue::Set(CA_ROW_ID),
            certificate_pem: ActiveValue::Set(certificate_pem.to_string()),
            key_pem: ActiveValue::Set(key_pem.to_string()),
            created_at: ActiveValue::Set(now),
            updated_at: ActiveValue::Set(now),
        };
        certificate_authority::Entity::insert(model)
            .exec(conn)
            .await
            .map_err(|e| anyhow!("storing the certificate authority: {e}"))?;
        Ok(())
    }
}
