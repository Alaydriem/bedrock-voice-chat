mod cached_token;
mod error;
mod token_format;

pub use cached_token::CachedToken;
pub use error::AccessTokenError;
pub use token_format::TokenFormat;

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::Duration;

use common::curia;
use common::ncryptflib::rocket::Utc;
use common::response::admin::{AccessTokenRow, LegacyTokenResponse, MintedTokenResponse};
use entity::game_access_token;
use sea_orm::{
    ActiveValue, ConnectionTrait, DatabaseConnection, EntityTrait, QueryOrder, TransactionTrait,
};
use tokio_util::sync::CancellationToken;

use crate::runtime::{SecretName, SecretStore};

/// How long a change made with `--local` can still be honoured by a running server.
///
/// A timer rather than a reload triggered by a failed match: a failed match propagates a
/// mint, because an unknown token misses, but never a revocation, because a revoked token
/// still matches its cached row and so never triggers the reload that would retire it.
const REFRESH_INTERVAL: Duration = Duration::from_secs(15);

/// Issues and verifies the credentials a game server presents as `Authorization: Bearer`.
pub struct AccessTokenService {
    db: Arc<DatabaseConnection>,
    tokens: RwLock<Arc<HashMap<String, CachedToken>>>,
    legacy: RwLock<Option<Arc<String>>>,
    legacy_configured: bool,
}

impl AccessTokenService {
    /// The id standing for the pre-identifier scalar in `server_secret`. `nanoid(8)` cannot
    /// produce a six-character value, so it can never collide with a minted id.
    pub const LEGACY_ID: &'static str = "legacy";

    pub fn new_shared(
        db: Arc<DatabaseConnection>,
        legacy: Option<String>,
        legacy_configured: bool,
    ) -> Arc<Self> {
        Arc::new(Self {
            db,
            tokens: RwLock::new(Arc::new(HashMap::new())),
            legacy: RwLock::new(legacy.map(Arc::new)),
            legacy_configured,
        })
    }

    /// True when the presented credential is live.
    ///
    /// Pure in-memory. An empty presentation is refused before any comparison: a bare
    /// `Authorization: Bearer ` header presents an empty string, and a constant-time
    /// comparison against an absent scalar would otherwise succeed.
    pub fn verify(&self, presented: &str) -> bool {
        if presented.is_empty() {
            return false;
        }

        if let Some((id, secret)) = TokenFormat::parse(presented) {
            let tokens = self.tokens.read().expect("token cache poisoned").clone();
            let Some(cached) = tokens.get(id) else {
                return false;
            };
            if cached.revoked_at.is_some() {
                return false;
            }
            return constant_time_eq::constant_time_eq(
                cached.secret_hash.as_bytes(),
                TokenFormat::hash(secret).as_bytes(),
            );
        }

        let legacy = self.legacy.read().expect("legacy cache poisoned").clone();
        match legacy {
            Some(value) if !value.is_empty() => {
                constant_time_eq::constant_time_eq(value.as_bytes(), presented.as_bytes())
            }
            _ => false,
        }
    }

    /// True when this deployment has issued no live credential of either kind.
    pub fn has_no_credential(&self) -> bool {
        let has_legacy = self
            .legacy
            .read()
            .expect("legacy cache poisoned")
            .as_ref()
            .is_some_and(|value| !value.is_empty());

        !has_legacy
            && self
                .tokens
                .read()
                .expect("token cache poisoned")
                .values()
                .all(|token| token.revoked_at.is_some())
    }

    /// Re-reads both credential sources into the cache.
    pub async fn reload(&self) -> Result<(), AccessTokenError> {
        let tokens = Self::load_in(self.db.as_ref()).await?;
        *self.tokens.write().expect("token cache poisoned") = Arc::new(tokens);

        // A configured scalar is re-applied at startup and cannot change under a running
        // process, so only a generated one is re-read.
        if !self.legacy_configured {
            let stored = SecretStore::read(self.db.as_ref(), SecretName::MinecraftAccessToken)
                .await
                .map_err(|e| AccessTokenError::Reload(e.to_string()))?;
            *self.legacy.write().expect("legacy cache poisoned") = stored.map(Arc::new);
        }

        Ok(())
    }

    /// Keeps the cache current so a change made with `--local` takes effect without a
    /// restart.
    pub fn spawn_refresh(self: &Arc<Self>, cancel: CancellationToken) {
        let service = Arc::clone(self);
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(REFRESH_INTERVAL);
            ticker.tick().await;
            loop {
                tokio::select! {
                    _ = cancel.cancelled() => return,
                    _ = ticker.tick() => {
                        if let Err(e) = service.reload().await {
                            curia::warn!("failed to refresh game access tokens", { "error": e.to_string() });
                        }
                    }
                }
            }
        });
    }

    pub async fn mint(&self) -> Result<MintedTokenResponse, AccessTokenError> {
        let minted = Self::mint_in(self.db.as_ref()).await?;
        self.reload().await?;
        Ok(minted)
    }

    pub async fn revoke(&self, id: &str) -> Result<bool, AccessTokenError> {
        if id == Self::LEGACY_ID {
            return self.revoke_legacy().await;
        }

        let revoked = Self::revoke_in(self.db.as_ref(), id).await?;
        self.reload().await?;
        Ok(revoked)
    }

    pub async fn rotate(&self, id: &str) -> Result<MintedTokenResponse, AccessTokenError> {
        let minted = Self::rotate_in(self.db.as_ref(), id).await?;
        self.reload().await?;
        Ok(minted)
    }

    pub fn legacy(&self) -> LegacyTokenResponse {
        let value = self.legacy.read().expect("legacy cache poisoned").clone();
        LegacyTokenResponse {
            token: value.map(|v| v.as_ref().clone()),
            configured: self.legacy_configured,
        }
    }

    /// Deletes the pre-identifier scalar. Refused when configuration supplies it, because
    /// startup would write it back.
    pub async fn revoke_legacy(&self) -> Result<bool, AccessTokenError> {
        if self.legacy_configured {
            return Err(AccessTokenError::LegacyIsConfigured);
        }

        let removed = SecretStore::delete(self.db.as_ref(), SecretName::MinecraftAccessToken)
            .await
            .map_err(|e| AccessTokenError::Reload(e.to_string()))?;
        self.reload().await?;
        Ok(removed)
    }

    /// Issues a credential. The returned token is the only copy of the secret.
    pub async fn mint_in<C: ConnectionTrait>(
        conn: &C,
    ) -> Result<MintedTokenResponse, AccessTokenError> {
        let (id, secret) = TokenFormat::mint();

        let model = game_access_token::ActiveModel {
            id: ActiveValue::Set(id.clone()),
            secret_hash: ActiveValue::Set(TokenFormat::hash(&secret)),
            created_at: ActiveValue::Set(Utc::now().timestamp()),
            revoked_at: ActiveValue::Set(None),
        };
        game_access_token::Entity::insert(model).exec(conn).await?;

        Ok(MintedTokenResponse {
            id: id.clone(),
            token: TokenFormat::compose(&id, &secret),
            revoked: None,
        })
    }

    /// Marks a credential revoked. Returns false when no such id exists.
    pub async fn revoke_in<C: ConnectionTrait>(
        conn: &C,
        id: &str,
    ) -> Result<bool, AccessTokenError> {
        let Some(row) = game_access_token::Entity::find_by_id(id.to_string())
            .one(conn)
            .await?
        else {
            return Ok(false);
        };

        if row.revoked_at.is_some() {
            return Ok(true);
        }

        let model = game_access_token::ActiveModel {
            id: ActiveValue::Unchanged(row.id),
            secret_hash: ActiveValue::NotSet,
            created_at: ActiveValue::NotSet,
            revoked_at: ActiveValue::Set(Some(Utc::now().timestamp())),
        };
        game_access_token::Entity::update(model).exec(conn).await?;

        Ok(true)
    }

    /// Issues a replacement and retires `id` in one transaction.
    ///
    /// Both halves or neither. A mint whose revoke failed would leave two live credentials
    /// and an operator who believes there is one.
    pub async fn rotate_in(
        conn: &DatabaseConnection,
        id: &str,
    ) -> Result<MintedTokenResponse, AccessTokenError> {
        let txn = conn.begin().await?;

        if !Self::revoke_in(&txn, id).await? {
            txn.rollback().await?;
            return Err(AccessTokenError::UnknownId(id.to_string()));
        }

        let mut minted = Self::mint_in(&txn).await?;
        txn.commit().await?;

        minted.revoked = Some(id.to_string());
        Ok(minted)
    }

    pub async fn list_in<C: ConnectionTrait>(
        conn: &C,
    ) -> Result<Vec<AccessTokenRow>, AccessTokenError> {
        let rows = game_access_token::Entity::find()
            .order_by_asc(game_access_token::Column::CreatedAt)
            .all(conn)
            .await?;

        Ok(rows
            .into_iter()
            .map(|row| AccessTokenRow {
                id: row.id,
                created_at: row.created_at,
                revoked_at: row.revoked_at,
            })
            .collect())
    }

    /// The verifiable state of every issued credential, keyed by id.
    pub async fn load_in<C: ConnectionTrait>(
        conn: &C,
    ) -> Result<HashMap<String, CachedToken>, AccessTokenError> {
        let rows = game_access_token::Entity::find().all(conn).await?;

        Ok(rows
            .into_iter()
            .map(|row| {
                (
                    row.id,
                    CachedToken {
                        secret_hash: row.secret_hash,
                        revoked_at: row.revoked_at,
                    },
                )
            })
            .collect())
    }
}
