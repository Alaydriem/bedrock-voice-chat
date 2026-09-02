//! Database-backed storage for the scalar server secrets.
//!
//! The database is the source of truth and the only place these values are read from at
//! runtime. Neither value is written to disk: nothing in the process opens them by path.

mod name;

pub use name::SecretName;

use common::curia;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use entity::server_secret;
use sea_orm::{ActiveValue, ConnectionTrait, EntityTrait};

pub struct SecretStore;

impl SecretStore {
    /// The stored value, if any.
    pub async fn read<C: ConnectionTrait>(conn: &C, name: SecretName) -> Result<Option<String>> {
        Ok(server_secret::Entity::find_by_id(name.as_str().to_string())
            .one(conn)
            .await?
            .map(|row| row.value))
    }

    /// Stores a value, replacing whatever was there.
    ///
    /// Distinct from `resolve`, which is for a secret this process may have to invent.
    /// A value handed to us by something else — the relay's assignment, for instance —
    /// has no generator and no legacy file, and reaching for `resolve` to write one
    /// would mean supplying a closure that must never run.
    pub async fn write<C: ConnectionTrait>(
        conn: &C,
        name: SecretName,
        value: &str,
    ) -> Result<()> {
        Self::persist(conn, name, value).await
    }

    /// Resolves a secret and leaves the database holding it.
    ///
    /// 1. A non-blank `configured` value is authoritative and is mirrored into the database.
    /// 2. Otherwise a stored row is used.
    /// 3. Otherwise a `legacy` file is imported. This is the upgrade path: a deployment that
    ///    predates the table has the value on disk and nothing in the database, and minting a
    ///    fresh one would discard an identity the deployment already published.
    /// 4. Otherwise one is generated.
    pub async fn resolve<C: ConnectionTrait>(
        conn: &C,
        name: SecretName,
        configured: Option<&str>,
        legacy: Option<&Path>,
        generate: impl FnOnce() -> String,
    ) -> Result<String> {
        if let Some(value) = configured.map(str::trim).filter(|v| !v.is_empty()) {
            Self::persist(conn, name, value).await?;
            return Ok(value.to_string());
        }

        if let Some(value) = Self::read(conn, name).await? {
            return Ok(value);
        }

        if let Some(path) = legacy
            && path.exists()
        {
            let value = fs::read_to_string(path)
                .with_context(|| format!("reading {}", path.display()))?
                .trim()
                .to_string();
            if !value.is_empty() {
                curia::info!("Importing an existing on-disk secret into the database. The file is no \
                     longer read and can be removed.", { "secret": name.as_str(), "path": path.display().to_string() });
                Self::persist(conn, name, &value).await?;
                return Ok(value);
            }
        }

        let value = generate();
        Self::persist(conn, name, &value).await?;
        Ok(value)
    }

    async fn persist<C: ConnectionTrait>(conn: &C, name: SecretName, value: &str) -> Result<()> {
        let now = common::ncryptflib::rocket::Utc::now().timestamp();
        let existing = server_secret::Entity::find_by_id(name.as_str().to_string())
            .one(conn)
            .await?;

        if let Some(row) = existing {
            if row.value == value {
                return Ok(());
            }
            let model = server_secret::ActiveModel {
                name: ActiveValue::Unchanged(row.name),
                value: ActiveValue::Set(value.to_string()),
                created_at: ActiveValue::NotSet,
                updated_at: ActiveValue::Set(now),
            };
            server_secret::Entity::update(model)
                .exec(conn)
                .await
                .with_context(|| format!("updating the {} secret", name.as_str()))?;
            return Ok(());
        }

        let model = server_secret::ActiveModel {
            name: ActiveValue::Set(name.as_str().to_string()),
            value: ActiveValue::Set(value.to_string()),
            created_at: ActiveValue::Set(now),
            updated_at: ActiveValue::Set(now),
        };
        server_secret::Entity::insert(model)
            .exec(conn)
            .await
            .with_context(|| format!("storing the {} secret", name.as_str()))?;
        Ok(())
    }

    /// Removes a stored secret. Returns false when there was nothing to remove.
    pub async fn delete<C: ConnectionTrait>(conn: &C, name: SecretName) -> Result<bool> {
        let result = server_secret::Entity::delete_by_id(name.as_str().to_string())
            .exec(conn)
            .await
            .with_context(|| format!("deleting the {} secret", name.as_str()))?;

        Ok(result.rows_affected > 0)
    }
}
