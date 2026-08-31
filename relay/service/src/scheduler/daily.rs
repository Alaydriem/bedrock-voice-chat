use std::sync::Arc;
use std::time::Duration;

use base64::Engine;
use common::curia;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use tokio_util::sync::CancellationToken;

use crate::config::DiscordConfig;
use crate::discord::MemberSource;
use crate::enroll::EnrollSessions;
use crate::entity::{RegistrationState, registration};
use crate::validation::{AddressProbe, RoutableAddress, ValidationChecker, ValidationError};

// One pass over every active registration, once a day.
//
// Three observations per registration, cheapest first: entitlement from the relay's
// own Discord credential, identity over the held session, and — only where an address
// record was published — the nonce fetched from that address.
pub struct DailyScheduler {
    conn: Arc<DatabaseConnection>,
    checker: Arc<ValidationChecker>,
    sessions: Arc<EnrollSessions>,
    members: MemberSource,
    discord: DiscordConfig,
    probe: AddressProbe,
}

impl DailyScheduler {
    pub const INTERVAL: Duration = Duration::from_secs(86_400);

    const NONCE_BYTES: usize = 32;

    pub fn new(
        conn: Arc<DatabaseConnection>,
        checker: Arc<ValidationChecker>,
        sessions: Arc<EnrollSessions>,
        members: MemberSource,
        discord: DiscordConfig,
        probe: AddressProbe,
    ) -> Self {
        Self {
            conn,
            checker,
            sessions,
            members,
            discord,
            probe,
        }
    }

    pub fn new_shared(
        conn: Arc<DatabaseConnection>,
        checker: Arc<ValidationChecker>,
        sessions: Arc<EnrollSessions>,
        members: MemberSource,
        discord: DiscordConfig,
        probe: AddressProbe,
    ) -> Arc<Self> {
        Arc::new(Self::new(conn, checker, sessions, members, discord, probe))
    }

    pub fn spawn(self: &Arc<Self>, cancel: CancellationToken) {
        let scheduler = Arc::clone(self);
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = cancel.cancelled() => break,
                    _ = tokio::time::sleep(Self::INTERVAL) => {
                        if let Err(e) = scheduler.run_once().await {
                            curia::error!(format!("the daily validation pass failed: {e}"));
                        }
                    }
                }
            }
        });
    }

    // How many registrations were evaluated. A pass over an empty registry is not an
    // error.
    pub async fn run_once(&self) -> Result<usize, ValidationError> {
        let rows = registration::Entity::find()
            .filter(registration::Column::State.eq(RegistrationState::Active.as_str()))
            .all(self.conn.as_ref())
            .await?;

        let mut evaluated = 0usize;
        for row in rows {
            self.evaluate_one(&row).await?;
            evaluated += 1;
        }

        Ok(evaluated)
    }

    async fn evaluate_one(&self, row: &registration::Model) -> Result<(), ValidationError> {
        let entitled = match self.members.role_ids(&row.discord_user_id).await {
            Ok(roles) => self.discord.qualifies(&roles),
            // An unreachable Discord is not a lapsed membership. Treating it as one
            // would suspend every registration during an outage that has nothing to
            // do with the operators.
            Err(e) => {
                curia::warn!(format!("skipping an entitlement check: {e}"), { "node": row.node_id.clone() });
                true
            }
        };

        let nonce = Self::mint_nonce();
        let identity_ok = entitled && self.answered_challenge(&row.node_id, &nonce).await;

        // Only a publicly routable declared address is probed. A private one is
        // unreachable from here by construction, and it cannot front anyone either —
        // it resolves only on the network of whoever declared it, which is the whole
        // case an operator behind NAT or CGNAT is in.
        let probeable = row
            .declared_address
            .as_deref()
            .filter(|address| RoutableAddress::is_public(address));

        let address_ok = match probeable {
            Some(address) if identity_ok => {
                Some(self.probe.serves_nonce(&row.name, address, &nonce).await)
            }
            // Identity already failed, so the address half cannot rescue the pass.
            // Recorded as failed rather than skipped so the outcome reads the same
            // whichever half went first.
            Some(_) => Some(false),
            None => None,
        };

        self.checker
            .evaluate(&row.node_id, identity_ok, address_ok)
            .await?;

        Ok(())
    }

    async fn answered_challenge(&self, node_id: &str, nonce: &str) -> bool {
        let Ok(key) = node_id.parse::<iroh::PublicKey>() else {
            return false;
        };

        let Some(signature) = self.sessions.challenge(&key, nonce.as_bytes()).await else {
            return false;
        };

        let Ok(bytes) = <[u8; 64]>::try_from(signature.as_slice()) else {
            return false;
        };

        key.verify(nonce.as_bytes(), &iroh::Signature::from_bytes(&bytes))
            .is_ok()
    }

    fn mint_nonce() -> String {
        let mut bytes = [0u8; Self::NONCE_BYTES];
        getrandom::fill(&mut bytes).expect("the system random source is available");
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
    }
}
