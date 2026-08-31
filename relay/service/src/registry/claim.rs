use std::sync::Arc;

use base64::Engine;
use sea_orm::{ActiveModelTrait, ActiveValue, DatabaseConnection, EntityTrait};

use crate::entity::claim;

use super::error::RegistryError;

// Carries one enrollment token across an origin boundary.
//
// The page that shows an operator their token is on a different origin from this
// registry, and the token must not reach it through a redirect URL, a fragment, or a
// cookie. A claim is the indirection: the redirect carries an id, the page exchanges
// the id for the token, and the id stops working immediately.
pub struct ClaimService {
    conn: Arc<DatabaseConnection>,
}

impl ClaimService {
    // The page redeems the moment it loads. A minute is generous for a redirect and
    // short enough that an id left in browser history is worthless.
    pub const TTL_SECONDS: i64 = 60;

    const ID_BYTES: usize = 32;

    pub fn new(conn: Arc<DatabaseConnection>) -> Self {
        Self { conn }
    }

    pub fn new_shared(conn: Arc<DatabaseConnection>) -> Arc<Self> {
        Arc::new(Self::new(conn))
    }

    pub async fn store(&self, token: &str) -> Result<String, RegistryError> {
        self.store_expiring_at(token, Self::now() + Self::TTL_SECONDS)
            .await
    }

    // Separate so a test can create one that has already expired without waiting a
    // minute for it.
    pub async fn store_expiring_at(
        &self,
        token: &str,
        expires_at: i64,
    ) -> Result<String, RegistryError> {
        let id = Self::mint_id();

        claim::ActiveModel {
            id: ActiveValue::Set(id.clone()),
            token: ActiveValue::Set(token.to_string()),
            expires_at: ActiveValue::Set(expires_at),
            consumed_at: ActiveValue::Set(None),
            created_at: ActiveValue::Set(Self::now()),
        }
        .insert(self.conn.as_ref())
        .await?;

        Ok(id)
    }

    // `None` for unknown, expired, or already-consumed. The page cannot tell them
    // apart and should not: every one of them means "this link is no longer good".
    //
    // Expiry is checked here rather than swept, so a claim stops working on time
    // whether or not anything else has run.
    pub async fn redeem(&self, id: &str) -> Result<Option<String>, RegistryError> {
        let Some(row) = claim::Entity::find_by_id(id).one(self.conn.as_ref()).await? else {
            return Ok(None);
        };

        if row.consumed_at.is_some() || row.expires_at < Self::now() {
            return Ok(None);
        }

        let token = row.token.clone();
        let mut model: claim::ActiveModel = row.into();
        model.consumed_at = ActiveValue::Set(Some(Self::now()));
        model.update(self.conn.as_ref()).await?;

        Ok(Some(token))
    }

    fn mint_id() -> String {
        let mut bytes = [0u8; Self::ID_BYTES];
        getrandom::fill(&mut bytes).expect("the system random source is available");
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
    }

    fn now() -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or_default()
    }
}
