use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result, anyhow};
use entity::acme_credential;
use sea_orm::{ActiveValue, DatabaseConnection, EntityTrait};

use super::expiry::CertificateExpiry;

const CERT_FILE: &str = "cert.pem";
const KEY_FILE: &str = "key.pem";
const ACCOUNT_FILE: &str = "account.json";

/// The single row's primary key. There is exactly one ACME account per deployment.
const ACME_ROW_ID: i32 = 1;

/// Database-backed persistence for ACME material, materialised under `<certs_path>/acme/`.
///
/// The database is the durable copy. The files exist because Rocket's `tls.certs` and
/// `tls.key` are paths, so a container needs no persistent volume for any of it — which
/// matters most here, where losing the account or the certificate costs a registration or a
/// re-issuance against a provider that rate-limits both.
pub struct AcmeStorage {
    dir: PathBuf,
    conn: Arc<DatabaseConnection>,
    directory_url: String,
    names: String,
}

impl AcmeStorage {
    pub fn new(
        certs_path: &str,
        conn: Arc<DatabaseConnection>,
        directory_url: String,
        names: Vec<String>,
    ) -> Self {
        // Sorted so the order the operator wrote the domains in is not mistaken for a change.
        let mut sorted = names;
        sorted.sort();
        Self {
            dir: PathBuf::from(certs_path).join("acme"),
            conn,
            directory_url,
            names: sorted.join(","),
        }
    }

    pub fn certificate_path(&self) -> PathBuf {
        self.dir.join(CERT_FILE)
    }

    pub fn key_path(&self) -> PathBuf {
        self.dir.join(KEY_FILE)
    }

    pub fn account_path(&self) -> PathBuf {
        self.dir.join(ACCOUNT_FILE)
    }

    /// Adopts material a pre-database deployment left on disk.
    ///
    /// Does nothing when a row already exists or when no `account.json` is present. The
    /// account is the expensive part: re-registering consumes a registration against a
    /// provider that caps them.
    pub async fn import_legacy(&self) -> Result<()> {
        if self.row().await?.is_some() {
            return Ok(());
        }
        let account_path = self.account_path();
        if !account_path.exists() {
            return Ok(());
        }

        let account_json = fs::read_to_string(&account_path)
            .with_context(|| format!("reading {}", account_path.display()))?;

        // A pair is adopted only when both halves are present. A certificate without its key
        // is unusable, and storing half would report a certificate that cannot serve.
        let pair = match (
            fs::read_to_string(self.certificate_path()).ok(),
            fs::read_to_string(self.key_path()).ok(),
        ) {
            (Some(cert), Some(key)) => (Some(cert), Some(key)),
            _ => (None, None),
        };

        tracing::info!(
            path = %self.dir.display(),
            "Importing the existing ACME account and certificate into the database."
        );

        self.insert(&account_json, pair.0, pair.1).await
    }

    /// The stored certificate, but only while it stays valid for at least `min_validity`, was
    /// issued by the configured provider, and covers the configured names. Anything else is
    /// treated as absent so the caller re-issues.
    pub async fn load_certificate_valid_for(
        &self,
        min_validity: Duration,
    ) -> Result<Option<String>> {
        let Some(row) = self.row().await? else {
            return Ok(None);
        };
        if row.directory_url != self.directory_url || row.names != self.names {
            return Ok(None);
        }
        let (Some(pem), Some(key_pem)) = (row.certificate_pem, row.key_pem) else {
            return Ok(None);
        };
        match CertificateExpiry::is_valid_for(&pem, min_validity) {
            Ok(true) => {
                // The consumer opens these by path, so a boot that serves the stored
                // certificate has to leave it on disk first.
                self.materialise(&pem, &key_pem)?;
                Ok(Some(pem))
            }
            // An unparseable or expiring stored certificate means re-issue, not crash.
            _ => Ok(None),
        }
    }

    pub async fn store_certificate(&self, cert_pem: &str, key_pem: &str) -> Result<()> {
        let Some(row) = self.row().await? else {
            return Err(anyhow!(
                "storing an ACME certificate before its account: the account row must exist first"
            ));
        };
        let model = acme_credential::ActiveModel {
            id: ActiveValue::Unchanged(row.id),
            account_json: ActiveValue::NotSet,
            certificate_pem: ActiveValue::Set(Some(cert_pem.to_string())),
            key_pem: ActiveValue::Set(Some(key_pem.to_string())),
            directory_url: ActiveValue::Set(self.directory_url.clone()),
            names: ActiveValue::Set(self.names.clone()),
            created_at: ActiveValue::NotSet,
            updated_at: ActiveValue::Set(Self::now()),
        };
        acme_credential::Entity::update(model)
            .exec(self.conn.as_ref())
            .await
            .context("storing the ACME certificate")?;

        self.materialise(cert_pem, key_pem)
    }

    pub async fn load_account_credentials(&self) -> Result<Option<String>> {
        Ok(self.row().await?.map(|row| row.account_json))
    }

    pub async fn store_account_credentials(&self, json: &str) -> Result<()> {
        match self.row().await? {
            Some(row) => {
                let model = acme_credential::ActiveModel {
                    id: ActiveValue::Unchanged(row.id),
                    account_json: ActiveValue::Set(json.to_string()),
                    certificate_pem: ActiveValue::NotSet,
                    key_pem: ActiveValue::NotSet,
                    directory_url: ActiveValue::Set(self.directory_url.clone()),
                    names: ActiveValue::NotSet,
                    created_at: ActiveValue::NotSet,
                    updated_at: ActiveValue::Set(Self::now()),
                };
                acme_credential::Entity::update(model)
                    .exec(self.conn.as_ref())
                    .await
                    .context("updating the ACME account")?;
                Ok(())
            }
            None => self.insert(json, None, None).await,
        }
    }

    async fn row(&self) -> Result<Option<acme_credential::Model>> {
        Ok(acme_credential::Entity::find_by_id(ACME_ROW_ID)
            .one(self.conn.as_ref())
            .await?)
    }

    async fn insert(
        &self,
        account_json: &str,
        certificate_pem: Option<String>,
        key_pem: Option<String>,
    ) -> Result<()> {
        let now = Self::now();
        let model = acme_credential::ActiveModel {
            id: ActiveValue::Set(ACME_ROW_ID),
            account_json: ActiveValue::Set(account_json.to_string()),
            certificate_pem: ActiveValue::Set(certificate_pem),
            key_pem: ActiveValue::Set(key_pem),
            directory_url: ActiveValue::Set(self.directory_url.clone()),
            names: ActiveValue::Set(self.names.clone()),
            created_at: ActiveValue::Set(now),
            updated_at: ActiveValue::Set(now),
        };
        acme_credential::Entity::insert(model)
            .exec(self.conn.as_ref())
            .await
            .context("storing the ACME account")?;
        Ok(())
    }

    fn materialise(&self, cert_pem: &str, key_pem: &str) -> Result<()> {
        fs::create_dir_all(&self.dir)
            .with_context(|| format!("creating {}", self.dir.display()))?;
        fs::write(self.certificate_path(), cert_pem)
            .with_context(|| format!("writing {}", self.certificate_path().display()))?;
        Self::write_restricted(&self.key_path(), key_pem)
    }

    fn now() -> i64 {
        common::ncryptflib::rocket::Utc::now().timestamp()
    }

    /// Owner-only permissions on Unix; Windows ACLs inherit from the certs directory, the
    /// same contract as ca.key. Renewal overwrites in place.
    fn write_restricted(path: &Path, content: &str) -> Result<()> {
        #[cfg(unix)]
        {
            use std::io::Write;
            use std::os::unix::fs::OpenOptionsExt;
            let mut file = fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .mode(0o600)
                .open(path)
                .with_context(|| format!("creating {}", path.display()))?;
            file.write_all(content.as_bytes())
                .with_context(|| format!("writing {}", path.display()))?;
        }
        #[cfg(not(unix))]
        {
            fs::write(path, content).with_context(|| format!("writing {}", path.display()))?;
        }
        Ok(())
    }
}
