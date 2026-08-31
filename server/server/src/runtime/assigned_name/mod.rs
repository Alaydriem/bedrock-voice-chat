//! The hostname the relay registry assigned this server.

use anyhow::Result;
use sea_orm::ConnectionTrait;

use super::secret_store::{SecretName, SecretStore};

// Held in the database beside the node key and resolved on every boot, for the same
// reason: a server that has enrolled once must start on its own name whether or not
// the relay answers. Losing it costs the operator their advertised address and every
// mod configuration pointing at it, and the name is retired rather than reissued.
pub struct AssignedNameStore;

impl AssignedNameStore {
    pub async fn read<C: ConnectionTrait>(conn: &C) -> Result<Option<String>> {
        SecretStore::read(conn, SecretName::RelayAssignedName).await
    }

    pub async fn write<C: ConnectionTrait>(conn: &C, name: &str) -> Result<()> {
        SecretStore::write(conn, SecretName::RelayAssignedName, name).await
    }
}
