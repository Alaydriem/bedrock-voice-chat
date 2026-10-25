use std::net::IpAddr;
use std::sync::Arc;

use base64::Engine;
use common::curia;
use sea_orm::sea_query::Expr;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, Condition, DatabaseConnection, EntityTrait,
    QueryFilter,
};

use crate::config::DiscordConfig;
use crate::discord::MemberSource;
use crate::entity::{RegistrationState, enrollment_token, registration};
use crate::naming::NameGenerator;

use super::check_in::CheckInRecord;
use super::error::RegistryError;
use super::token::EnrollmentToken;

// Every decision about who holds which name.
//
// The transport is not here: this is called by the enrollment endpoint and by the
// web UI, and both need the same answers.
pub struct RegistryService {
    conn: Arc<DatabaseConnection>,
    discord: DiscordConfig,
    members: MemberSource,
    names: NameGenerator,
}

impl RegistryService {
    const NONCE_BYTES: usize = 32;

    pub fn new(
        conn: Arc<DatabaseConnection>,
        discord: DiscordConfig,
        members: MemberSource,
    ) -> Self {
        Self {
            conn,
            discord,
            members,
            names: NameGenerator::new(),
        }
    }

    pub fn new_shared(
        conn: Arc<DatabaseConnection>,
        discord: DiscordConfig,
        members: MemberSource,
    ) -> Arc<Self> {
        Arc::new(Self::new(conn, discord, members))
    }

    pub async fn issue_token(&self, discord_user_id: &str) -> Result<String, RegistryError> {
        let roles = self.members.role_ids(discord_user_id).await?;
        if !self.discord.qualifies(&roles) {
            return Err(RegistryError::NotEntitled);
        }

        let existing = registration::Entity::find()
            .filter(registration::Column::DiscordUserId.eq(discord_user_id))
            .one(self.conn.as_ref())
            .await?;

        if let Some(row) = existing
            && RegistrationState::from_str(&row.state) != Some(RegistrationState::Retired)
        {
            return Err(RegistryError::AlreadyRegistered);
        }

        let token = EnrollmentToken::mint();
        let now = Self::now();

        enrollment_token::ActiveModel {
            token: ActiveValue::Set(token.clone()),
            discord_user_id: ActiveValue::Set(discord_user_id.to_string()),
            expires_at: ActiveValue::Set(now + EnrollmentToken::TTL_SECONDS),
            redeemed_at: ActiveValue::Set(None),
            redeemed_by_node_id: ActiveValue::Set(None),
            created_at: ActiveValue::Set(now),
        }
        .insert(self.conn.as_ref())
        .await?;

        Ok(token)
    }

    pub async fn redeem(&self, token: &str, node_id: &str) -> Result<String, RegistryError> {
        let row = enrollment_token::Entity::find_by_id(token)
            .one(self.conn.as_ref())
            .await?
            .ok_or(RegistryError::UnknownToken)?;

        if row.redeemed_at.is_some() {
            return Err(RegistryError::TokenAlreadyRedeemed);
        }

        let now = Self::now();
        if row.expires_at < now {
            return Err(RegistryError::UnknownToken);
        }

        // Re-checked at redemption rather than trusted from issuance. A membership
        // can lapse between the two, and the token is valid for a day.
        let roles = self.members.role_ids(&row.discord_user_id).await?;
        if !self.discord.qualifies(&roles) {
            return Err(RegistryError::NotEntitled);
        }

        let name = self.names.assign(self.conn.as_ref()).await?;

        registration::ActiveModel {
            node_id: ActiveValue::Set(node_id.to_string()),
            name: ActiveValue::Set(name.clone()),
            discord_user_id: ActiveValue::Set(row.discord_user_id.clone()),
            state: ActiveValue::Set(RegistrationState::Active.as_str().to_string()),
            declared_address: ActiveValue::Set(None),
            address_verified_at: ActiveValue::Set(None),
            entitlement_checked_at: ActiveValue::Set(Some(now)),
            entitlement_ok: ActiveValue::Set(true),
            validated_at: ActiveValue::Set(None),
            validation_failures: ActiveValue::Set(0),
            created_at: ActiveValue::Set(now),
            suspended_at: ActiveValue::Set(None),
            retired_at: ActiveValue::Set(None),
            revoked_at: ActiveValue::Set(None),
            declared_port: ActiveValue::Set(None),
            checkin_nonce: ActiveValue::Set(None),
            checked_in_at: ActiveValue::Set(None),
        }
        .insert(self.conn.as_ref())
        .await?;

        let mut spent: enrollment_token::ActiveModel = row.into();
        spent.redeemed_at = ActiveValue::Set(Some(now));
        spent.redeemed_by_node_id = ActiveValue::Set(Some(node_id.to_string()));
        spent.update(self.conn.as_ref()).await?;

        Ok(name)
    }

    // A revoked registration keeps its name so its server can still renew its certificate.
    pub async fn name_for(&self, node_id: &str) -> Result<Option<String>, RegistryError> {
        let row = registration::Entity::find_by_id(node_id)
            .one(self.conn.as_ref())
            .await?;

        Ok(
            row.and_then(|row| match RegistrationState::from_str(&row.state) {
                Some(RegistrationState::Active) => Some(row.name),
                Some(RegistrationState::Revoked) if row.entitlement_ok => Some(row.name),
                _ => None,
            }),
        )
    }

    // `entitled` is `None` when Discord could not be reached; the last known answer stands.
    pub async fn record_entitlement_check(
        &self,
        node_id: &str,
        entitled: Option<bool>,
    ) -> Result<(), RegistryError> {
        let row = registration::Entity::find_by_id(node_id)
            .one(self.conn.as_ref())
            .await?
            .ok_or(RegistryError::NotRegistered)?;

        let mut model: registration::ActiveModel = row.into();
        model.entitlement_checked_at = ActiveValue::Set(Some(Self::now()));
        if let Some(entitled) = entitled {
            model.entitlement_ok = ActiveValue::Set(entitled);
        }
        model.update(self.conn.as_ref()).await?;

        Ok(())
    }

    // A changed address or port clears `address_verified_at`.
    pub async fn record_check_in(
        &self,
        node_id: &str,
        address: Option<IpAddr>,
        port: u16,
    ) -> Result<CheckInRecord, RegistryError> {
        let row = registration::Entity::find_by_id(node_id)
            .one(self.conn.as_ref())
            .await?
            .ok_or(RegistryError::NotRegistered)?;

        let state = match RegistrationState::from_str(&row.state) {
            Some(
                state @ (RegistrationState::Active
                | RegistrationState::Pending
                | RegistrationState::Revoked),
            ) => state,
            _ => {
                curia::warn!("refusing a check-in from a registration in an unusable state", { "node": node_id.to_string(), "state": row.state.clone() });
                return Err(RegistryError::Suspended);
            }
        };

        let address = address.map(|a| a.to_string());
        let address_changed =
            row.declared_address != address || row.declared_port != Some(i32::from(port));
        let nonce = Self::mint_nonce();
        let name = row.name.clone();

        let mut model: registration::ActiveModel = row.into();
        model.declared_address = ActiveValue::Set(address);
        model.declared_port = ActiveValue::Set(Some(i32::from(port)));
        model.checkin_nonce = ActiveValue::Set(Some(nonce.clone()));
        model.checked_in_at = ActiveValue::Set(Some(Self::now()));
        if address_changed {
            model.address_verified_at = ActiveValue::Set(None);
        }
        model.update(self.conn.as_ref()).await?;

        Ok(CheckInRecord {
            name,
            state,
            nonce,
            address_changed,
        })
    }

    // Atomic: concurrent check-ins win the window at most once.
    pub async fn claim_recovery_evaluation(
        &self,
        node_id: &str,
        cooldown_seconds: i64,
    ) -> Result<bool, RegistryError> {
        let now = Self::now();
        let result = registration::Entity::update_many()
            .col_expr(
                registration::Column::EntitlementCheckedAt,
                Expr::value(Some(now)),
            )
            .filter(registration::Column::NodeId.eq(node_id))
            .filter(
                Condition::any()
                    .add(registration::Column::EntitlementCheckedAt.is_null())
                    .add(registration::Column::EntitlementCheckedAt.lte(now - cooldown_seconds)),
            )
            .exec(self.conn.as_ref())
            .await?;
        Ok(result.rows_affected == 1)
    }

    pub async fn revoke(&self, node_id: &str) -> Result<(), RegistryError> {
        let row = registration::Entity::find_by_id(node_id)
            .one(self.conn.as_ref())
            .await?
            .ok_or(RegistryError::NotRegistered)?;

        let mut model: registration::ActiveModel = row.into();
        model.state = ActiveValue::Set(RegistrationState::Revoked.as_str().to_string());
        model.revoked_at = ActiveValue::Set(Some(Self::now()));
        model.update(self.conn.as_ref()).await?;

        Ok(())
    }

    pub async fn reinstate(&self, node_id: &str) -> Result<(), RegistryError> {
        let row = registration::Entity::find_by_id(node_id)
            .one(self.conn.as_ref())
            .await?
            .ok_or(RegistryError::NotRegistered)?;

        let mut model: registration::ActiveModel = row.into();
        model.state = ActiveValue::Set(RegistrationState::Active.as_str().to_string());
        model.revoked_at = ActiveValue::Set(None);
        model.update(self.conn.as_ref()).await?;

        Ok(())
    }

    fn mint_nonce() -> String {
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
