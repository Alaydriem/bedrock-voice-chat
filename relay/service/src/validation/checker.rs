use std::net::IpAddr;
use std::sync::Arc;

use common::curia;
use sea_orm::{ActiveModelTrait, ActiveValue, DatabaseConnection, EntityTrait};

use super::error::ValidationError;
use crate::dns::ZoneWriter;
use crate::entity::{RegistrationState, registration};
use crate::registry::{RegistryError, RegistryService};

use super::outcome::ValidationOutcome;

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
        let state = match RegistrationState::from_str(&row.state) {
            Some(state @ (RegistrationState::Active | RegistrationState::Revoked)) => state,
            _ => {
                return Ok(if passed {
                    ValidationOutcome::Passed
                } else {
                    ValidationOutcome::Failed {
                        consecutive: row.validation_failures,
                    }
                });
            }
        };

        let declared_address = row.declared_address.clone();

        // A pass clears the counter rather than decrementing it. Failures that
        // accumulate across unrelated outages weeks apart are not evidence of an
        // abandoned server.
        if passed {
            let mut model: registration::ActiveModel = row.into();
            model.validated_at = ActiveValue::Set(Some(now));
            model.validation_failures = ActiveValue::Set(0);
            model.update(self.conn.as_ref()).await?;

            if state != RegistrationState::Revoked {
                return Ok(ValidationOutcome::Passed);
            }

            // Published first so a failed write leaves the row revoked and retried.
            if let Some(address) = declared_address.and_then(|a| a.parse::<IpAddr>().ok()) {
                self.zone.publish_address(&name, address).await?;
            }
            self.registry.reinstate(node_id).await?;
            curia::info!("recovered a revoked registration", { "node": node_id.to_string(), "name": name });
            return Ok(ValidationOutcome::Recovered);
        }

        let mut model: registration::ActiveModel = row.into();
        model.validation_failures = ActiveValue::Set(consecutive);
        model.update(self.conn.as_ref()).await?;

        if state == RegistrationState::Revoked || consecutive < Self::FAILURE_THRESHOLD {
            return Ok(ValidationOutcome::Failed { consecutive });
        }

        // Challenge records stay so a revoked server can still renew.
        curia::warn!("revoking a registration after repeated validation failures", { "node": node_id.to_string(), "name": name.clone() });
        self.zone.withdraw_address(&name).await?;
        self.registry.revoke(node_id).await?;

        Ok(ValidationOutcome::Revoked)
    }

    fn now() -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or_default()
    }
}
