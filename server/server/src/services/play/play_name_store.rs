use anyhow::Result;
use sea_orm::ConnectionTrait;

use crate::runtime::{SecretName, SecretStore};

// Held in the database for the same reason as the assigned name: the registry binds the
// name to this server's node key, and a server that has claimed one must start on it
// whether or not the registry answers.
pub struct PlayNameStore;

impl PlayNameStore {
    pub async fn read<C: ConnectionTrait>(conn: &C) -> Result<Option<String>> {
        SecretStore::read(conn, SecretName::PlayName).await
    }

    pub async fn write<C: ConnectionTrait>(conn: &C, name: &str) -> Result<()> {
        SecretStore::write(conn, SecretName::PlayName, name).await
    }

    // Forgets the name, so the next boot claims one again. For a name the registry no
    // longer binds to this server's key.
    pub async fn clear<C: ConnectionTrait>(conn: &C) -> Result<bool> {
        SecretStore::delete(conn, SecretName::PlayName).await
    }
}
