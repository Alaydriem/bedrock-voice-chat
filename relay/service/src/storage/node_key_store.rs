use std::sync::Arc;

use sea_orm::{ActiveModelTrait, ActiveValue, DatabaseConnection, EntityTrait};

use crate::entity::node_key;

use super::error::StorageError;

// The registry's iroh secret key, held in the database.
//
// Durable state rather than a session value: this key IS the registry's identity, and
// every enrolled server holds a peer link naming it. Regenerating it silently would
// make every one of them dial a node that no longer answers.
//
// In the database rather than in a file so the container needs no volume — the database
// is then the only thing to back up, and losing it is unambiguously fatal rather than
// half-fatal in a way nobody notices until a server tries to reach the registry.
pub struct NodeKeyStore {
    conn: Arc<DatabaseConnection>,
}

impl NodeKeyStore {
    // Named so a second durable secret becomes another row rather than another table.
    const NAME: &'static str = "registry-node-key";

    const KEY_LEN: usize = 32;

    pub fn new(conn: Arc<DatabaseConnection>) -> Self {
        Self { conn }
    }

    pub fn new_shared(conn: Arc<DatabaseConnection>) -> Arc<Self> {
        Arc::new(Self::new(conn))
    }

    // Reads the stored key, generating and storing one on the first start.
    //
    // A stored value that is not 32 bytes is an error rather than a cue to make a fresh
    // one. Replacing it would change the registry's identity, which is exactly the
    // failure this type exists to prevent — and it would do so at the moment the
    // operator is least likely to notice.
    pub async fn resolve(&self) -> Result<[u8; Self::KEY_LEN], StorageError> {
        if let Some(row) = node_key::Entity::find_by_id(Self::NAME)
            .one(self.conn.as_ref())
            .await?
        {
            return Self::decode(&row.value);
        }

        let mut bytes = [0u8; Self::KEY_LEN];
        getrandom::fill(&mut bytes).expect("the system random source is available");

        node_key::ActiveModel {
            name: ActiveValue::Set(Self::NAME.to_string()),
            value: ActiveValue::Set(Self::encode(&bytes)),
            created_at: ActiveValue::Set(Self::now()),
        }
        .insert(self.conn.as_ref())
        .await?;

        Ok(bytes)
    }

    fn now() -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or_default()
    }

    fn encode(bytes: &[u8; Self::KEY_LEN]) -> String {
        bytes.iter().map(|b| format!("{b:02x}")).collect()
    }

    fn decode(hex: &str) -> Result<[u8; Self::KEY_LEN], StorageError> {
        if hex.len() != Self::KEY_LEN * 2 {
            return Err(StorageError::MalformedNodeKey(hex.len()));
        }

        let mut bytes = [0u8; Self::KEY_LEN];
        for (i, byte) in bytes.iter_mut().enumerate() {
            *byte = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16)
                .map_err(|_| StorageError::MalformedNodeKey(hex.len()))?;
        }

        Ok(bytes)
    }
}
