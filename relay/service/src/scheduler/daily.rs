use std::sync::Arc;
use std::time::Duration;

use common::curia;
use sea_orm::{ColumnTrait, Condition, DatabaseConnection, EntityTrait, QueryFilter};
use tokio_util::sync::CancellationToken;

use crate::config::DiscordConfig;
use crate::discord::MemberSource;
use crate::dns::ZoneWriter;
use crate::entity::{RegistrationState, registration};
use crate::registry::{ClaimService, RegistryEndpoint, RegistryError, RegistryService};
use crate::validation::{
    AddressVerifier, LiveAddressProbe, RoutableAddress, ValidationChecker, ValidationError,
};

// The cadence is stored per registration, so a relay redeployed more often than
// daily still validates.
pub struct DailyScheduler {
    conn: Arc<DatabaseConnection>,
    checker: Arc<ValidationChecker>,
    verifier: Arc<AddressVerifier>,
    registry: Arc<RegistryService>,
    zone: Arc<ZoneWriter>,
    claims: Arc<ClaimService>,
    members: MemberSource,
    discord: DiscordConfig,
}

impl DailyScheduler {
    pub const INTERVAL: Duration = Duration::from_secs(86_400);

    // The first tick fires immediately, so a restart catches up.
    pub const TICK: Duration = Duration::from_secs(3_600);

    #[allow(clippy::too_many_arguments)]
    pub fn new(
        conn: Arc<DatabaseConnection>,
        checker: Arc<ValidationChecker>,
        verifier: Arc<AddressVerifier>,
        registry: Arc<RegistryService>,
        zone: Arc<ZoneWriter>,
        claims: Arc<ClaimService>,
        members: MemberSource,
        discord: DiscordConfig,
    ) -> Self {
        Self {
            conn,
            checker,
            verifier,
            registry,
            zone,
            claims,
            members,
            discord,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new_shared(
        conn: Arc<DatabaseConnection>,
        checker: Arc<ValidationChecker>,
        verifier: Arc<AddressVerifier>,
        registry: Arc<RegistryService>,
        zone: Arc<ZoneWriter>,
        claims: Arc<ClaimService>,
        members: MemberSource,
        discord: DiscordConfig,
    ) -> Arc<Self> {
        Arc::new(Self::new(
            conn, checker, verifier, registry, zone, claims, members, discord,
        ))
    }

    pub fn spawn(self: &Arc<Self>, cancel: CancellationToken) {
        let scheduler = Arc::clone(self);
        tokio::spawn(async move {
            let mut tick = tokio::time::interval(Self::TICK);
            tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            loop {
                tokio::select! {
                    _ = cancel.cancelled() => break,
                    _ = tick.tick() => scheduler.tick(Self::now()).await,
                }
            }
        });
    }

    pub async fn tick(&self, now: i64) {
        self.prune_challenges(now).await;
        self.withdraw_suspended().await;
        self.withdraw_revoked_addresses().await;
        self.sweep_claims(now).await;
        match self.run_due(now).await {
            Ok(evaluated) if evaluated > 0 => {
                curia::info!("validated registrations that were due", { "evaluated": evaluated });
            }
            Ok(_) => {}
            Err(e) => curia::error!("the validation pass failed", { "error": e.to_string() }),
        }
    }

    pub async fn run_once(&self) -> Result<usize, ValidationError> {
        self.run(Condition::all(), Self::now()).await
    }

    pub async fn run_due(&self, now: i64) -> Result<usize, ValidationError> {
        let cutoff = now - Self::interval_seconds();
        self.run(
            Condition::any()
                .add(registration::Column::EntitlementCheckedAt.is_null())
                .add(registration::Column::EntitlementCheckedAt.lte(cutoff)),
            now,
        )
        .await
    }

    pub async fn evaluate_node(&self, node_id: &str) -> Result<(), ValidationError> {
        let row = registration::Entity::find_by_id(node_id)
            .one(self.conn.as_ref())
            .await?
            .ok_or(RegistryError::NotRegistered)?;
        self.evaluate_one(&row, Self::now()).await
    }

    async fn prune_challenges(&self, now: i64) {
        let cutoff = now - RegistryEndpoint::CHALLENGE_TTL_SECONDS;
        if let Err(e) = self.zone.prune_challenges_before(cutoff).await {
            curia::warn!("pruning expired challenge records failed", { "error": e.to_string() });
        }
    }

    async fn withdraw_suspended(&self) {
        let rows = match registration::Entity::find()
            .filter(registration::Column::State.is_not_in([
                RegistrationState::Pending.as_str(),
                RegistrationState::Active.as_str(),
                RegistrationState::Revoked.as_str(),
                RegistrationState::Retired.as_str(),
            ]))
            .all(self.conn.as_ref())
            .await
        {
            Ok(rows) => rows,
            Err(e) => {
                curia::warn!("listing suspended registrations failed", { "error": e.to_string() });
                return;
            }
        };

        for row in rows {
            let withdrawn = match self.zone.has_records(&row.name).await {
                Ok(true) => self.zone.withdraw(&row.name).await,
                Ok(false) => continue,
                Err(e) => Err(e),
            };
            if let Err(e) = withdrawn {
                curia::warn!("withdrawing a suspended registration's records failed", { "node": row.node_id.clone(), "error": e.to_string() });
            }
        }
    }

    // A check-in in flight when a node is revoked can republish its address after the revocation withdrew it.
    async fn withdraw_revoked_addresses(&self) {
        let rows = match registration::Entity::find()
            .filter(registration::Column::State.eq(RegistrationState::Revoked.as_str()))
            .all(self.conn.as_ref())
            .await
        {
            Ok(rows) => rows,
            Err(e) => {
                curia::warn!("listing revoked registrations failed", { "error": e.to_string() });
                return;
            }
        };

        for row in rows {
            let withdrawn = match self.zone.has_address_record(&row.name).await {
                Ok(true) => self.zone.withdraw_address(&row.name).await,
                Ok(false) => continue,
                Err(e) => Err(e),
            };
            if let Err(e) = withdrawn {
                curia::warn!("withdrawing a revoked registration's address failed", { "node": row.node_id.clone(), "error": e.to_string() });
            }
        }
    }

    async fn sweep_claims(&self, now: i64) {
        if let Err(e) = self.claims.sweep(now).await {
            curia::warn!("sweeping enrollment claims failed", { "error": e.to_string() });
        }
    }

    // One registration's failure is logged and skipped so the rest still run.
    async fn run(&self, due: Condition, now: i64) -> Result<usize, ValidationError> {
        let rows = registration::Entity::find()
            .filter(registration::Column::State.is_in([
                RegistrationState::Active.as_str(),
                RegistrationState::Revoked.as_str(),
            ]))
            .filter(due)
            .all(self.conn.as_ref())
            .await?;

        let mut evaluated = 0usize;
        for row in rows {
            match self.evaluate_one(&row, now).await {
                Ok(()) => evaluated += 1,
                Err(e) => {
                    curia::error!("validating a registration failed", { "node": row.node_id.clone(), "error": e.to_string() });
                }
            }
        }

        Ok(evaluated)
    }

    async fn evaluate_one(&self, row: &registration::Model, now: i64) -> Result<(), ValidationError> {
        let fresh = match self.members.role_ids(&row.discord_user_id).await {
            Ok(roles) => Some(self.discord.qualifies(&roles)),
            // An unreachable Discord is not a lapsed membership.
            Err(e) => {
                curia::warn!("skipping an entitlement check", { "node": row.node_id.clone(), "error": e.to_string() });
                None
            }
        };
        self.registry
            .record_entitlement_check(&row.node_id, fresh)
            .await?;
        let entitled = fresh.unwrap_or(row.entitlement_ok);

        let checked_in = row
            .checked_in_at
            .is_some_and(|at| at >= now - Self::interval_seconds());
        let identity_ok = entitled && checked_in;

        // A private address is unreachable from here and fronts nobody, so only a
        // public one is probed.
        let public = row
            .declared_address
            .as_deref()
            .filter(|address| RoutableAddress::is_public(address));

        let address_ok = match (public, row.checkin_nonce.as_deref()) {
            (Some(address), Some(nonce)) if identity_ok => {
                let port = row
                    .declared_port
                    .and_then(|port| u16::try_from(port).ok())
                    .unwrap_or(LiveAddressProbe::DEFAULT_PORT);
                Some(
                    self.verifier
                        .serves_nonce(&row.name, address, port, nonce)
                        .await,
                )
            }
            (Some(_), _) if !identity_ok => Some(false),
            _ => None,
        };

        self.checker
            .evaluate(&row.node_id, identity_ok, address_ok)
            .await?;

        Ok(())
    }

    fn interval_seconds() -> i64 {
        Self::INTERVAL.as_secs() as i64
    }

    fn now() -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or_default()
    }
}
