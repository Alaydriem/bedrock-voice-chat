mod error;
mod outcome;

pub use error::PairingError;
pub use outcome::RedeemOutcome;

use std::time::Duration;

use chrono::Utc;
use common::structs::relay::{Capability, PairingCode};
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter,
    TransactionSession, TransactionTrait,
};

/// Mints and redeems the codes that authorize a peer.
///
/// Every method is an associated function taking a connection rather than a method on a
/// stored handle: redemption runs inside a transaction the caller owns, and a service
/// holding its own connection cannot join one.
pub struct PairingService;

impl PairingService {
    /// Wrong guesses a code tolerates before its window is spent.
    ///
    /// The code is 40 bits, so this is not what makes guessing infeasible. It is what stops
    /// a bridge with a mistyped code from retrying indefinitely against a live window.
    pub const MAX_ATTEMPTS: i32 = 5;

    /// Long enough to walk between two terminals, short enough that a code forgotten in a
    /// scrollback is not a standing credential.
    pub const DEFAULT_TTL: Duration = Duration::from_secs(15 * 60);

    pub async fn mint<C: ConnectionTrait>(
        conn: &C,
        label: &str,
        ttl: Duration,
    ) -> Result<String, PairingError> {
        let (plaintext, code) = PairingCode::generate();
        let now = Utc::now().timestamp();

        let row = entity::peer_pairing_code::ActiveModel {
            code_digest: ActiveValue::Set(code.to_hex()),
            label: ActiveValue::Set(label.to_string()),
            expires_at: ActiveValue::Set(now + ttl.as_secs() as i64),
            consumed_at: ActiveValue::Set(None),
            attempts: ActiveValue::Set(0),
            created_at: ActiveValue::Set(now),
        };

        entity::peer_pairing_code::Entity::insert(row)
            .exec(conn)
            .await?;

        Ok(plaintext)
    }

    /// Redeems a code for a node, or reports why it could not be.
    ///
    /// The grant write and the `consumed_at` stamp share one transaction. Two concurrent
    /// redemptions that both read `consumed_at = None` before either writes would both be
    /// approved, and single-use would be defeated with no test going red.
    pub async fn redeem<C: ConnectionTrait + TransactionTrait>(
        conn: &C,
        node_id: &str,
        code: &str,
        declared: &[String],
    ) -> Result<RedeemOutcome, PairingError> {
        if let Some(existing) = entity::peer_grant::Entity::find_by_id(node_id.to_string())
            .one(conn)
            .await?
        {
            let (worlds, capabilities) = Self::scope_of(&existing)?;

            return Ok(RedeemOutcome::AlreadyPaired {
                label: existing.label.clone(),
                worlds: Self::narrow(&worlds, declared),
                capabilities,
            });
        }

        let txn = conn.begin().await?;

        let digest = PairingCode::from_plaintext(code).to_hex();
        let now = Utc::now().timestamp();

        let Some(row) = entity::peer_pairing_code::Entity::find_by_id(digest)
            .one(&txn)
            .await?
        else {
            Self::charge_every_live_code(&txn, now).await?;
            txn.commit().await?;

            return Ok(RedeemOutcome::Unknown);
        };

        // Consumption and expiry are checked before anything is written, so a dead code is
        // never compared further and never costs an attempt against a live one.
        if row.consumed_at.is_some() || row.attempts >= Self::MAX_ATTEMPTS {
            txn.commit().await?;
            return Ok(RedeemOutcome::Spent);
        }

        if row.expires_at <= now {
            txn.commit().await?;
            return Ok(RedeemOutcome::Expired);
        }

        let label = row.label.clone();
        let capabilities = vec![Capability::CarrySpeakers];

        let mut code_row: entity::peer_pairing_code::ActiveModel = row.into();
        code_row.consumed_at = ActiveValue::Set(Some(now));
        code_row.update(&txn).await?;

        let grant = entity::peer_grant::ActiveModel {
            node_id: ActiveValue::Set(node_id.to_string()),
            label: ActiveValue::Set(label.clone()),
            // Empty: a paired grant narrows nothing, so the peer's own declaration stands.
            worlds: ActiveValue::Set(String::new()),
            capabilities: ActiveValue::Set(
                capabilities
                    .iter()
                    .map(|c| c.as_str().to_string())
                    .collect::<Vec<_>>()
                    .join(","),
            ),
            paired_at: ActiveValue::Set(now),
        };
        grant.insert(&txn).await?;

        txn.commit().await?;

        Ok(RedeemOutcome::Paired {
            label,
            worlds: declared.to_vec(),
            capabilities,
        })
    }

    pub async fn revoke<C: ConnectionTrait>(conn: &C, label: &str) -> Result<u64, PairingError> {
        let result = entity::peer_grant::Entity::delete_many()
            .filter(entity::peer_grant::Column::Label.eq(label))
            .exec(conn)
            .await?;

        Ok(result.rows_affected)
    }

    pub async fn paired<C: ConnectionTrait>(
        conn: &C,
    ) -> Result<Vec<entity::peer_grant::Model>, PairingError> {
        Ok(entity::peer_grant::Entity::find().all(conn).await?)
    }

    // A wrong code is indistinguishable from an unknown one — the lookup is by digest, so
    // there is no row to charge. Every live code takes the attempt instead, which is what
    // makes a guessing run terminate.
    async fn charge_every_live_code<C: ConnectionTrait>(
        conn: &C,
        now: i64,
    ) -> Result<(), PairingError> {
        let live = entity::peer_pairing_code::Entity::find()
            .filter(entity::peer_pairing_code::Column::ConsumedAt.is_null())
            .filter(entity::peer_pairing_code::Column::ExpiresAt.gt(now))
            .all(conn)
            .await?;

        for row in live {
            let attempts = row.attempts + 1;
            let mut active: entity::peer_pairing_code::ActiveModel = row.into();
            active.attempts = ActiveValue::Set(attempts);
            active.update(conn).await?;
        }

        Ok(())
    }

    fn scope_of(
        row: &entity::peer_grant::Model,
    ) -> Result<(Vec<String>, Vec<Capability>), PairingError> {
        let worlds = row
            .worlds
            .split(',')
            .filter(|w| !w.is_empty())
            .map(str::to_string)
            .collect();

        let mut capabilities = Vec::new();
        for tag in row.capabilities.split(',').filter(|c| !c.is_empty()) {
            let capability = Capability::from_tag(tag)
                .ok_or_else(|| PairingError::Corrupt(format!("unknown capability {tag:?}")))?;
            capabilities.push(capability);
        }

        Ok((worlds, capabilities))
    }

    fn narrow(filter: &[String], declared: &[String]) -> Vec<String> {
        if filter.is_empty() {
            return declared.to_vec();
        }

        declared
            .iter()
            .filter(|world| filter.iter().any(|f| f == *world))
            .cloned()
            .collect()
    }
}
