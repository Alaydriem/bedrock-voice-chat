use std::sync::Arc;

use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter,
};

use crate::config::DiscordConfig;
use crate::discord::MemberSource;
use crate::entity::{RegistrationState, enrollment_token, registration};
use crate::naming::NameGenerator;

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
        }
        .insert(self.conn.as_ref())
        .await?;

        let mut spent: enrollment_token::ActiveModel = row.into();
        spent.redeemed_at = ActiveValue::Set(Some(now));
        spent.redeemed_by_node_id = ActiveValue::Set(Some(node_id.to_string()));
        spent.update(self.conn.as_ref()).await?;

        Ok(name)
    }

    // The name this node may act for, or `None` when it holds no live registration.
    //
    // A suspended registration answers `None`: it keeps its row so recovery is a
    // state change, but it must not authorize a DNS write in the meantime.
    pub async fn name_for(&self, node_id: &str) -> Result<Option<String>, RegistryError> {
        let row = registration::Entity::find_by_id(node_id)
            .one(self.conn.as_ref())
            .await?;

        Ok(row.and_then(
            |row| match RegistrationState::from_str(&row.state) {
                Some(RegistrationState::Active) => Some(row.name),
                _ => None,
            },
        ))
    }

    // The address this node last declared, if any. `None` for a node that has never
    // declared one, which is the ordinary case for a server nothing but BVC clients
    // reach.
    pub async fn declared_address(&self, node_id: &str) -> Result<Option<String>, RegistryError> {
        Ok(registration::Entity::find_by_id(node_id)
            .one(self.conn.as_ref())
            .await?
            .and_then(|row| row.declared_address))
    }

    // Records the address an operator says their server answers on, and returns the
    // name it belongs to.
    //
    // Persisted rather than only published, because the daily pass reads this column
    // to decide whether to bind the record to the node. An address published without
    // being recorded is one nothing ever verifies.
    //
    // `address_verified_at` is cleared: a newly declared address has not been checked
    // yet, and carrying the previous one's verification forward would vouch for a host
    // nobody has looked at.
    pub async fn declare_address(
        &self,
        node_id: &str,
        address: &str,
    ) -> Result<String, RegistryError> {
        let row = registration::Entity::find_by_id(node_id)
            .one(self.conn.as_ref())
            .await?
            .ok_or(RegistryError::NotRegistered)?;

        if RegistrationState::from_str(&row.state) != Some(RegistrationState::Active) {
            return Err(RegistryError::Suspended);
        }

        let name = row.name.clone();
        let mut model: registration::ActiveModel = row.into();
        model.declared_address = ActiveValue::Set(Some(address.to_string()));
        model.address_verified_at = ActiveValue::Set(None);
        model.update(self.conn.as_ref()).await?;

        Ok(name)
    }

    pub async fn suspend(&self, node_id: &str) -> Result<(), RegistryError> {
        let row = registration::Entity::find_by_id(node_id)
            .one(self.conn.as_ref())
            .await?
            .ok_or(RegistryError::NotRegistered)?;

        let mut model: registration::ActiveModel = row.into();
        model.state = ActiveValue::Set(RegistrationState::Suspended.as_str().to_string());
        model.suspended_at = ActiveValue::Set(Some(Self::now()));
        model.update(self.conn.as_ref()).await?;

        Ok(())
    }

    fn now() -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or_default()
    }
}
