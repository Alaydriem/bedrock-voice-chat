use std::sync::Arc;

use common::curia;
use sea_orm::{ActiveModelTrait, ActiveValue, DatabaseConnection, EntityTrait};

use crate::dns::ZoneWriter;
use crate::entity::registration;
use crate::registry::{RegistryError, RegistryService};
use super::error::ValidationError;

use super::outcome::ValidationOutcome;

// Decides whether a registration keeps its name.
//
// One rule for every registration. Identity and liveness are proved over the held
// enrollment session, which reaches a node behind CGNAT exactly as it reaches one
// with a public address; the address half applies only where a record was published.
pub struct ValidationChecker {
    conn: Arc<DatabaseConnection>,
    registry: Arc<RegistryService>,
    zone: Arc<ZoneWriter>,
}

impl ValidationChecker {
    // Consecutive failures before a name is withdrawn. Three daily checks is a
    // deliberate margin: one missed check is an outage, and a name withdrawn for an
    // outage costs the operator their advertised address.
    pub const FAILURE_THRESHOLD: i32 = 3;

    pub fn new(
        conn: Arc<DatabaseConnection>,
        registry: Arc<RegistryService>,
        zone: Arc<ZoneWriter>,
    ) -> Self {
        Self {
            conn,
            registry,
            zone,
        }
    }

    pub fn new_shared(
        conn: Arc<DatabaseConnection>,
        registry: Arc<RegistryService>,
        zone: Arc<ZoneWriter>,
    ) -> Arc<Self> {
        Arc::new(Self::new(conn, registry, zone))
    }

    // `address_ok` is `None` for a registration that published no address record.
    pub async fn evaluate(
        &self,
        node_id: &str,
        identity_ok: bool,
        address_ok: Option<bool>,
    ) -> Result<ValidationOutcome, ValidationError> {
        let row = registration::Entity::find_by_id(node_id)
            .one(self.conn.as_ref())
            .await?
            .ok_or(RegistryError::NotRegistered)?;

        let name = row.name.clone();
        let consecutive = row.validation_failures + 1;
        let passed = identity_ok && address_ok.unwrap_or(true);
        let now = Self::now();

        // A pass clears the counter rather than decrementing it. Failures that
        // accumulate across unrelated outages weeks apart are not evidence of an
        // abandoned server.
        if passed {
            let mut model: registration::ActiveModel = row.into();
            model.validated_at = ActiveValue::Set(Some(now));
            model.validation_failures = ActiveValue::Set(0);
            model.update(self.conn.as_ref()).await?;
            return Ok(ValidationOutcome::Passed);
        }

        let mut model: registration::ActiveModel = row.into();
        model.validation_failures = ActiveValue::Set(consecutive);
        model.update(self.conn.as_ref()).await?;

        if consecutive < Self::FAILURE_THRESHOLD {
            return Ok(ValidationOutcome::Failed { consecutive });
        }

        // Suspension withdraws the record and pauses renewal. It never revokes: the
        // live certificate stays valid until it expires, so the remaining lifetime
        // is the operator's grace period.
        curia::warn!("suspending a registration after repeated validation failures", { "node": node_id.to_string(), "name": name.clone() });
        self.zone.withdraw(&name).await?;
        self.registry.suspend(node_id).await?;

        Ok(ValidationOutcome::Suspended)
    }

    fn now() -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or_default()
    }
}
