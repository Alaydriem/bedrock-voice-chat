use std::sync::Arc;
use std::time::Duration;

use common::curia;
use sea_orm::{ColumnTrait, Condition, DatabaseConnection, EntityTrait, QueryFilter};
use tokio_util::sync::CancellationToken;

use crate::config::DiscordConfig;
use crate::discord::MemberSource;
use crate::dns::ZoneWriter;
use crate::entity::{RegistrationState, registration};
use crate::registry::{RegistryEndpoint, RegistryService};
use crate::validation::{AddressVerifier, RoutableAddress, ValidationChecker, ValidationError};

// Validates every active registration once a day.
//
// Three observations per registration, cheapest first: entitlement from the relay's
// own Discord credential, identity over the held session, and — only where a public
// address was declared — the nonce fetched from that address.
//
// The cadence is kept per registration, in the database. A timer that slept a day
// from process start would never fire on a relay redeployed more often than daily,
// and validation would silently stop.
pub struct DailyScheduler {
    conn: Arc<DatabaseConnection>,
    checker: Arc<ValidationChecker>,
    verifier: Arc<AddressVerifier>,
    registry: Arc<RegistryService>,
    zone: Arc<ZoneWriter>,
    members: MemberSource,
    discord: DiscordConfig,
}

impl DailyScheduler {
    // How often a registration is validated.
    pub const INTERVAL: Duration = Duration::from_secs(86_400);

    // How often the scheduler looks for registrations that are due. The first look
    // is immediate, so a restart catches up rather than waiting out a full tick.
    pub const TICK: Duration = Duration::from_secs(3_600);

    pub fn new(
        conn: Arc<DatabaseConnection>,
        checker: Arc<ValidationChecker>,
        verifier: Arc<AddressVerifier>,
        registry: Arc<RegistryService>,
        zone: Arc<ZoneWriter>,
        members: MemberSource,
        discord: DiscordConfig,
    ) -> Self {
        Self {
            conn,
            checker,
            verifier,
            registry,
            zone,
            members,
            discord,
        }
    }

    pub fn new_shared(
        conn: Arc<DatabaseConnection>,
        checker: Arc<ValidationChecker>,
        verifier: Arc<AddressVerifier>,
        registry: Arc<RegistryService>,
        zone: Arc<ZoneWriter>,
        members: MemberSource,
        discord: DiscordConfig,
    ) -> Arc<Self> {
        Arc::new(Self::new(
            conn, checker, verifier, registry, zone, members, discord,
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
                    _ = tick.tick() => {
                        let now = Self::now();
                        scheduler.prune_challenges(now).await;
                        match scheduler.run_due(now).await {
                            Ok(evaluated) if evaluated > 0 => {
                                curia::info!("validated registrations that were due", { "evaluated": evaluated });
                            }
                            Ok(_) => {}
                            Err(e) => curia::error!(format!("the validation pass failed: {e}")),
                        }
                    }
                }
            }
        });
    }

    // Evaluates every active registration regardless of when it was last checked.
    // Returns how many were evaluated; a pass over an empty registry is not an error.
    pub async fn run_once(&self) -> Result<usize, ValidationError> {
        self.run(Condition::all()).await
    }

    // Evaluates the active registrations last checked a full interval before `now`,
    // or never.
    pub async fn run_due(&self, now: i64) -> Result<usize, ValidationError> {
        let cutoff = now - Self::INTERVAL.as_secs() as i64;
        self.run(
            Condition::any()
                .add(registration::Column::EntitlementCheckedAt.is_null())
                .add(registration::Column::EntitlementCheckedAt.lte(cutoff)),
        )
        .await
    }

    // Challenge records past their lifetime. Pruned here as well as on publish, so a
    // name that stops issuing does not keep its last records forever.
    async fn prune_challenges(&self, now: i64) {
        let cutoff = now - RegistryEndpoint::CHALLENGE_TTL_SECONDS;
        if let Err(e) = self.zone.prune_challenges_before(cutoff).await {
            curia::warn!(format!("pruning expired challenge records failed: {e}"));
        }
    }

    // One registration's failure is logged and passed over. Propagating it would end
    // the pass, and every registration after it would go unvalidated — including the
    // ones that should be suspended.
    async fn run(&self, due: Condition) -> Result<usize, ValidationError> {
        let rows = registration::Entity::find()
            .filter(registration::Column::State.eq(RegistrationState::Active.as_str()))
            .filter(due)
            .all(self.conn.as_ref())
            .await?;

        let mut evaluated = 0usize;
        for row in rows {
            match self.evaluate_one(&row).await {
                Ok(()) => evaluated += 1,
                Err(e) => {
                    curia::error!(format!("validating a registration failed: {e}"), { "node": row.node_id.clone() });
                }
            }
        }

        Ok(evaluated)
    }

    async fn evaluate_one(&self, row: &registration::Model) -> Result<(), ValidationError> {
        let entitled = match self.members.role_ids(&row.discord_user_id).await {
            Ok(roles) => Some(self.discord.qualifies(&roles)),
            // An unreachable Discord is not a lapsed membership. Treating it as one
            // would suspend every registration during an outage that has nothing to
            // do with the operators.
            Err(e) => {
                curia::warn!(format!("skipping an entitlement check: {e}"), { "node": row.node_id.clone() });
                None
            }
        };
        self.registry
            .record_entitlement_check(&row.node_id, entitled)
            .await?;

        let nonce = match entitled {
            Some(false) => None,
            _ => self.verifier.challenge(&row.node_id).await,
        };
        let identity_ok = nonce.is_some();

        // Only a publicly routable declared address is probed. A private one is
        // unreachable from here by construction, and it cannot front anyone either —
        // it resolves only on the network of whoever declared it, which is the whole
        // case an operator behind NAT or CGNAT is in.
        let probeable = row
            .declared_address
            .as_deref()
            .filter(|address| RoutableAddress::is_public(address));

        let address_ok = match (probeable, nonce.as_deref()) {
            (Some(address), Some(nonce)) => {
                Some(self.verifier.serves_nonce(&row.name, address, nonce).await)
            }
            // Identity already failed, so the address half cannot rescue the pass.
            // Recorded as failed rather than skipped so the outcome reads the same
            // whichever half went first.
            (Some(_), None) => Some(false),
            (None, _) => None,
        };

        self.checker
            .evaluate(&row.node_id, identity_ok, address_ok)
            .await?;

        Ok(())
    }

    fn now() -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or_default()
    }
}
