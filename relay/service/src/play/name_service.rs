use std::net::IpAddr;
use std::sync::Arc;

use base64::Engine;
use common::curia;
use common::structs::network::AddressClass;
use sea_orm::sea_query::Expr;
use tokio::sync::Mutex;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, Condition, ConnectionTrait, DatabaseConnection,
    EntityTrait, PaginatorTrait, QueryFilter, TransactionTrait,
};

use crate::config::PlayConfig;
use crate::entity::{play_issuance, play_name, play_zone_state};
use crate::naming::NameGenerator;

use super::check_in::PlayCheckIn;
use super::error::PlayError;

// Who holds which play name, and the zone serial those names move.
//
// Every change to a published record advances the serial in the same transaction as
// the record. A secondary ignores a zone whose serial is not larger than the one it
// holds, so a serial that lagged the table would leave secondaries on old data.
pub struct PlayNameService {
    conn: Arc<DatabaseConnection>,
    config: PlayConfig,
    names: NameGenerator,
    // Held from the limit check to the insert, so concurrent claims cannot all pass a
    // check that only the first of them would. One registry process owns the table.
    claim_lock: Mutex<()>,
    // Held from the daily count to the insert, for the same reason.
    issuance_lock: Mutex<()>,
}

impl PlayNameService {
    const NONCE_BYTES: usize = 32;
    const DAY_SECONDS: i64 = 86_400;
    const HOUR_SECONDS: i64 = 3_600;
    // Draws before giving up on a label that another claim took first.
    const CLAIM_ATTEMPTS: usize = 3;
    // The one row of play_zone_state.
    const STATE_ID: i32 = 1;

    pub fn new(conn: Arc<DatabaseConnection>, config: PlayConfig) -> Self {
        Self {
            conn,
            config,
            names: NameGenerator::new(),
            claim_lock: Mutex::new(()),
            issuance_lock: Mutex::new(()),
        }
    }

    pub fn new_shared(conn: Arc<DatabaseConnection>, config: PlayConfig) -> Arc<Self> {
        Arc::new(Self::new(conn, config))
    }

    // The fully qualified name a server is reached at.
    pub fn fqdn(&self, label: &str) -> String {
        format!("{label}.{}", self.config.zone)
    }

    // The label for a fully qualified name in this zone, or `None` for a name outside it.
    pub fn label_of(&self, fqdn: &str) -> Option<String> {
        fqdn.strip_suffix(&format!(".{}", self.config.zone))
            .filter(|label| !label.is_empty() && !label.contains('.'))
            .map(str::to_string)
    }

    // Returns the bare label. A node that already holds a live name gets it back, so a
    // retry or a fast restart never spends a second name.
    pub async fn claim(&self, node_id: &str, observed: IpAddr) -> Result<String, PlayError> {
        if let Some(label) = self.name_for(node_id).await? {
            return Ok(label);
        }

        let _guard = self.claim_lock.lock().await;

        // A claim from this node may have finished while this one waited.
        if let Some(label) = self.name_for(node_id).await? {
            return Ok(label);
        }

        self.enforce_limits(observed).await?;

        let mut last_error = None;
        for _ in 0..Self::CLAIM_ATTEMPTS {
            let label = self.names.assign_play(self.conn.as_ref()).await?;

            match self.store_claim(node_id, &label, observed).await {
                Ok(()) => {
                    curia::info!("Play name assigned", { "name": label.clone() });
                    return Ok(label);
                }
                Err(e) => {
                    // A concurrent claim from this node, or another node drawing the
                    // same label. The first is answered by the stored name.
                    if let Some(stored) = self.name_for(node_id).await? {
                        return Ok(stored);
                    }
                    last_error = Some(e);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| PlayError::Dns("no claim attempt was made".to_string())))
    }

    // `None` when the node holds no name or released it.
    pub async fn name_for(&self, node_id: &str) -> Result<Option<String>, PlayError> {
        let row = play_name::Entity::find_by_id(node_id)
            .one(self.conn.as_ref())
            .await?;

        Ok(row.filter(|row| row.released_at.is_none()).map(|row| row.name))
    }

    pub async fn record_check_in(
        &self,
        node_id: &str,
        address: IpAddr,
        port: u16,
    ) -> Result<PlayCheckIn, PlayError> {
        let row = Self::live_row(self.conn.as_ref(), node_id).await?;

        if !self.config.allow_private_probe_targets {
            match AddressClass::of(address) {
                AddressClass::Public => {}
                AddressClass::Cgnat => {
                    curia::info!("Play check-in refused: address is behind CGNAT", { "node": node_id.to_string(), "address": address.to_string() });
                    return Err(PlayError::AddressCgnat(address));
                }
                _ => {
                    curia::info!("Play check-in refused: address is not public", { "node": node_id.to_string(), "address": address.to_string() });
                    return Err(PlayError::AddressRefused(address));
                }
            }
        }

        let now = Self::now();
        let window = i64::try_from(self.config.min_check_in_interval_secs).unwrap_or(i64::MAX);
        if let (Some(at), Some(nonce)) = (row.checked_in_at, row.checkin_nonce.clone())
            && now.saturating_sub(at) < window
        {
            return Ok(PlayCheckIn {
                name: row.name,
                nonce,
                address_changed: false,
                throttled: true,
            });
        }

        let address_changed = row.published_address.as_deref() != Some(&address.to_string());
        let nonce = Self::mint_nonce();
        let name = row.name.clone();

        let mut model: play_name::ActiveModel = row.into();
        model.declared_address = ActiveValue::Set(Some(address.to_string()));
        model.declared_port = ActiveValue::Set(Some(i32::from(port)));
        model.checkin_nonce = ActiveValue::Set(Some(nonce.clone()));
        model.checked_in_at = ActiveValue::Set(Some(now));
        model.update(self.conn.as_ref()).await?;

        Ok(PlayCheckIn {
            name,
            nonce,
            address_changed,
            throttled: false,
        })
    }

    // Counts a certificate request against the node's daily allowance. Called before the
    // CA is asked, so a request the CA refuses still counts: it costs the CA a token.
    pub async fn admit_issuance(&self, node_id: &str) -> Result<(), PlayError> {
        let _guard = self.issuance_lock.lock().await;
        let now = Self::now();

        let issued = play_issuance::Entity::find()
            .filter(play_issuance::Column::NodeId.eq(node_id))
            .filter(play_issuance::Column::IssuedAt.gte(now - Self::DAY_SECONDS))
            .count(self.conn.as_ref())
            .await?;
        if issued >= u64::from(self.config.issuances_per_node_per_day) {
            curia::warn!("Play certificate refused: daily limit reached", { "node": node_id.to_string() });
            return Err(PlayError::RateLimited);
        }

        play_issuance::ActiveModel {
            id: ActiveValue::NotSet,
            node_id: ActiveValue::Set(node_id.to_string()),
            issued_at: ActiveValue::Set(now),
        }
        .insert(self.conn.as_ref())
        .await?;

        Ok(())
    }

    // Returns the zone serial. Publishing the address already published leaves the
    // serial where it is, so a node that checks in every few minutes does not make
    // every secondary re-transfer the zone.
    //
    // `nonce` is the check-in the probe belonged to. The row changes only while that is
    // still the node's latest check-in with that address; a probe from an earlier one
    // finds it moved on and changes nothing. `None` means the check-in was superseded.
    pub async fn mark_published(
        &self,
        node_id: &str,
        address: IpAddr,
        nonce: &str,
    ) -> Result<Option<u32>, PlayError> {
        let txn = self.conn.begin().await?;
        let row = Self::live_row(&txn, node_id).await?;
        let text = address.to_string();
        let unchanged = row.published_address.as_deref() == Some(text.as_str());

        let result = play_name::Entity::update_many()
            .col_expr(play_name::Column::PublishedAddress, Expr::value(text.clone()))
            .col_expr(play_name::Column::VerifiedAt, Expr::value(Some(Self::now())))
            .filter(play_name::Column::NodeId.eq(node_id))
            .filter(play_name::Column::CheckinNonce.eq(nonce))
            .filter(play_name::Column::DeclaredAddress.eq(text))
            .filter(play_name::Column::ReleasedAt.is_null())
            .exec(&txn)
            .await?;
        if result.rows_affected == 0 {
            return Ok(None);
        }

        let serial = if unchanged {
            Self::stored_serial(&txn).await?
        } else {
            Self::advance_serial(&txn).await?
        };
        txn.commit().await?;

        Ok(Some(serial))
    }

    // Returns the zone serial, or `None` when the check-in was superseded. Withdrawing a
    // name that is not published leaves the serial where it is.
    pub async fn mark_withdrawn(
        &self,
        node_id: &str,
        nonce: &str,
    ) -> Result<Option<u32>, PlayError> {
        let txn = self.conn.begin().await?;
        let row = Self::live_row(&txn, node_id).await?;
        let was_published = row.published_address.is_some();

        let result = play_name::Entity::update_many()
            .col_expr(
                play_name::Column::PublishedAddress,
                Expr::value(Option::<String>::None),
            )
            .col_expr(
                play_name::Column::VerifiedAt,
                Expr::value(Option::<i64>::None),
            )
            .filter(play_name::Column::NodeId.eq(node_id))
            .filter(play_name::Column::CheckinNonce.eq(nonce))
            .filter(play_name::Column::ReleasedAt.is_null())
            .exec(&txn)
            .await?;
        if result.rows_affected == 0 {
            return Ok(None);
        }

        let serial = if was_published {
            Self::advance_serial(&txn).await?
        } else {
            Self::stored_serial(&txn).await?
        };
        txn.commit().await?;

        Ok(Some(serial))
    }

    // Every record the zone serves: label and address.
    pub async fn published(&self) -> Result<Vec<(String, IpAddr)>, PlayError> {
        let rows = play_name::Entity::find()
            .filter(play_name::Column::ReleasedAt.is_null())
            .filter(play_name::Column::PublishedAddress.is_not_null())
            .all(self.conn.as_ref())
            .await?;

        Ok(rows
            .into_iter()
            .filter_map(|row| {
                let address = row.published_address?.parse::<IpAddr>().ok()?;
                Some((row.name, address))
            })
            .collect())
    }

    pub async fn serial(&self) -> Result<u32, PlayError> {
        Self::stored_serial(self.conn.as_ref()).await
    }

    // Releases every name with no check-in inside the retention window and retires its
    // label. Returns the released labels and the zone serial after the release.
    pub async fn release_stale(&self) -> Result<(Vec<String>, u32), PlayError> {
        let now = Self::now();
        let cutoff = now - i64::from(self.config.release_after_days) * Self::DAY_SECONDS;

        let txn = self.conn.begin().await?;

        let stale = play_name::Entity::find()
            .filter(play_name::Column::ReleasedAt.is_null())
            .filter(
                Condition::any()
                    .add(play_name::Column::CheckedInAt.lt(cutoff))
                    .add(
                        Condition::all()
                            .add(play_name::Column::CheckedInAt.is_null())
                            .add(play_name::Column::CreatedAt.lt(cutoff)),
                    ),
            )
            .all(&txn)
            .await?;

        let mut released = Vec::with_capacity(stale.len());
        let mut changed_zone = false;
        for row in stale {
            let label = row.name.clone();
            changed_zone |= row.published_address.is_some();

            let mut model: play_name::ActiveModel = row.into();
            model.released_at = ActiveValue::Set(Some(now));
            model.published_address = ActiveValue::Set(None);
            model.verified_at = ActiveValue::Set(None);
            model.update(&txn).await?;

            NameGenerator::retire(&txn, &label).await?;
            released.push(label);
        }

        play_issuance::Entity::delete_many()
            .filter(play_issuance::Column::IssuedAt.lt(now - 2 * Self::DAY_SECONDS))
            .exec(&txn)
            .await?;

        let serial = if changed_zone {
            Self::advance_serial(&txn).await?
        } else {
            Self::stored_serial(&txn).await?
        };
        txn.commit().await?;

        Ok((released, serial))
    }

    // The key the per-address limit counts by. An IPv6 host holds a whole /64, so the
    // prefix is the bucket; an IPv4-mapped address counts as the IPv4 address.
    fn bucket(address: IpAddr) -> String {
        match address.to_canonical() {
            IpAddr::V4(v4) => v4.to_string(),
            IpAddr::V6(v6) => {
                let s = v6.segments();
                format!("{:x}:{:x}:{:x}:{:x}::/64", s[0], s[1], s[2], s[3])
            }
        }
    }

    async fn enforce_limits(&self, observed: IpAddr) -> Result<(), PlayError> {
        let now = Self::now();

        let from_address = play_name::Entity::find()
            .filter(play_name::Column::ClaimedFrom.eq(Self::bucket(observed)))
            .filter(play_name::Column::CreatedAt.gte(now - Self::DAY_SECONDS))
            .count(self.conn.as_ref())
            .await?;
        if from_address >= u64::from(self.config.claims_per_address_per_day) {
            curia::warn!("Play claim refused: per-address limit reached", { "address": observed.to_string() });
            return Err(PlayError::RateLimited);
        }

        let in_hour = play_name::Entity::find()
            .filter(play_name::Column::CreatedAt.gte(now - Self::HOUR_SECONDS))
            .count(self.conn.as_ref())
            .await?;
        if in_hour >= u64::from(self.config.claims_per_hour) {
            curia::warn!("Play claim refused: hourly limit reached", { "address": observed.to_string() });
            return Err(PlayError::RateLimited);
        }

        Ok(())
    }

    // Inserts the node's row, or reassigns the row of a node that released its name.
    // A released label stays in its old row's history through `retired_name`, so the
    // node's primary key can be reused without the label ever being.
    async fn store_claim(
        &self,
        node_id: &str,
        label: &str,
        observed: IpAddr,
    ) -> Result<(), PlayError> {
        let now = Self::now();

        let existing = play_name::Entity::find_by_id(node_id)
            .one(self.conn.as_ref())
            .await?;

        if existing.is_some() {
            let result = play_name::Entity::update_many()
                .col_expr(play_name::Column::Name, Expr::value(label.to_string()))
                .col_expr(
                    play_name::Column::ClaimedFrom,
                    Expr::value(Self::bucket(observed)),
                )
                .col_expr(play_name::Column::CreatedAt, Expr::value(now))
                .col_expr(
                    play_name::Column::ReleasedAt,
                    Expr::value(Option::<i64>::None),
                )
                .col_expr(
                    play_name::Column::PublishedAddress,
                    Expr::value(Option::<String>::None),
                )
                .col_expr(
                    play_name::Column::DeclaredAddress,
                    Expr::value(Option::<String>::None),
                )
                .col_expr(
                    play_name::Column::DeclaredPort,
                    Expr::value(Option::<i32>::None),
                )
                .col_expr(
                    play_name::Column::VerifiedAt,
                    Expr::value(Option::<i64>::None),
                )
                .col_expr(
                    play_name::Column::CheckedInAt,
                    Expr::value(Option::<i64>::None),
                )
                .filter(play_name::Column::NodeId.eq(node_id))
                .filter(play_name::Column::ReleasedAt.is_not_null())
                .exec(self.conn.as_ref())
                .await?;

            if result.rows_affected == 0 {
                return Err(PlayError::Dns("the play name changed under a claim".into()));
            }

            return Ok(());
        }

        play_name::ActiveModel {
            node_id: ActiveValue::Set(node_id.to_string()),
            name: ActiveValue::Set(label.to_string()),
            claimed_from: ActiveValue::Set(Self::bucket(observed)),
            declared_address: ActiveValue::Set(None),
            declared_port: ActiveValue::Set(None),
            published_address: ActiveValue::Set(None),
            verified_at: ActiveValue::Set(None),
            checkin_nonce: ActiveValue::Set(None),
            checked_in_at: ActiveValue::Set(None),
            created_at: ActiveValue::Set(now),
            released_at: ActiveValue::Set(None),
        }
        .insert(self.conn.as_ref())
        .await?;

        Ok(())
    }

    async fn live_row<C: ConnectionTrait>(
        conn: &C,
        node_id: &str,
    ) -> Result<play_name::Model, PlayError> {
        play_name::Entity::find_by_id(node_id)
            .one(conn)
            .await?
            .filter(|row| row.released_at.is_none())
            .ok_or(PlayError::NoPlayName)
    }

    async fn stored_serial<C: ConnectionTrait>(conn: &C) -> Result<u32, PlayError> {
        let state = play_zone_state::Entity::find_by_id(Self::STATE_ID)
            .one(conn)
            .await?
            .ok_or_else(|| PlayError::Dns("the zone state row is missing".into()))?;

        Ok(state.serial as u32)
    }

    // `max(stored + 1, unix_now)`, applied as a compare-and-set so two changes at once
    // never hand out the same serial.
    async fn advance_serial<C: ConnectionTrait>(conn: &C) -> Result<u32, PlayError> {
        loop {
            let state = play_zone_state::Entity::find_by_id(Self::STATE_ID)
                .one(conn)
                .await?
                .ok_or_else(|| PlayError::Dns("the zone state row is missing".into()))?;

            let next = (state.serial + 1).max(Self::now());
            let result = play_zone_state::Entity::update_many()
                .col_expr(play_zone_state::Column::Serial, Expr::value(next))
                .filter(play_zone_state::Column::Id.eq(Self::STATE_ID))
                .filter(play_zone_state::Column::Serial.eq(state.serial))
                .exec(conn)
                .await?;

            if result.rows_affected == 1 {
                return Ok(next as u32);
            }
        }
    }

    pub(crate) fn mint_nonce() -> String {
        let mut bytes = [0u8; Self::NONCE_BYTES];
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
