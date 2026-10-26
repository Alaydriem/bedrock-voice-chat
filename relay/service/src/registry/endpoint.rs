use std::net::IpAddr;
use std::sync::Arc;

use bvc_relay::node::NodeIdentity;
use bvc_relay::peer::{AddressObserver, AdmissionControl, PeerEndpoint};
use common::curia;
use common::structs::relay::enroll::{EnrollFrame, EnrollRefuseReason, EnrollVersion};
use common::structs::relay::wire::Framing;
use iroh::PublicKey;
use iroh::endpoint::{Connection, RecvStream, SendStream};

use crate::budget::WeeklyBudget;
use crate::dns::ZoneWriter;
use crate::entity::RegistrationState;
use crate::registry::RegistryService;
use crate::scheduler::DailyScheduler;

use crate::enroll::EnrollError;

pub struct RegistryEndpoint {
    endpoint: PeerEndpoint,
    registry: Arc<RegistryService>,
    zone: Arc<ZoneWriter>,
    budget: Arc<WeeklyBudget>,
    scheduler: Arc<DailyScheduler>,
    admission: AdmissionControl,
}

impl RegistryEndpoint {
    pub const ALPN: &'static [u8] = b"bvc-enroll/1";

    // Sized above the peer plane's 64 so enrollments cannot starve voice peering.
    pub const MAX_UNAUTHORIZED: usize = 256;

    pub const CHALLENGE_TTL_SECONDS: i64 = 60 * 60;

    pub const MAX_OPEN_CHALLENGES: u64 = 4;

    // Caps Discord lookups and probes from a reconnect loop.
    pub const RECOVERY_COOLDOWN_SECONDS: i64 = 600;

    pub async fn bind(
        identity: &NodeIdentity,
        registry: Arc<RegistryService>,
        zone: Arc<ZoneWriter>,
        budget: Arc<WeeklyBudget>,
        scheduler: Arc<DailyScheduler>,
        port: Option<u16>,
    ) -> Result<Arc<Self>, EnrollError> {
        let endpoint = PeerEndpoint::bind_with_alpns(
            identity,
            port,
            vec![Self::ALPN.to_vec(), AddressObserver::ALPN.to_vec()],
        )
        .await
        .map_err(|e| EnrollError::Bind(e.to_string()))?;

        Ok(Arc::new(Self {
            endpoint,
            registry,
            zone,
            budget,
            scheduler,
            admission: AdmissionControl::new(Self::MAX_UNAUTHORIZED),
        }))
    }

    pub fn node_id(&self) -> PublicKey {
        self.endpoint.node_id()
    }

    pub async fn ticket(&self) -> Result<String, EnrollError> {
        self.endpoint
            .ticket()
            .await
            .map_err(|e| EnrollError::Bind(e.to_string()))
    }

    pub fn spawn_accept_loop(self: &Arc<Self>) {
        let endpoint = Arc::clone(self);
        tokio::spawn(async move {
            while let Some(incoming) = endpoint.endpoint.endpoint().accept().await {
                let endpoint = Arc::clone(&endpoint);
                tokio::spawn(async move {
                    endpoint.admit(incoming).await;
                });
            }
        });
    }

    async fn admit(self: &Arc<Self>, incoming: iroh::endpoint::Incoming) {
        let Some(slot) = self.admission.try_admit() else {
            curia::warn!("refusing a registry connection: too many handshakes in flight");
            return;
        };

        // Iroh exposes the remote address on `Incoming`, not on `Connection`.
        let observed = incoming.remote_addr();

        let conn = match incoming.await {
            Ok(conn) => conn,
            Err(e) => {
                curia::debug!("a registry connection failed before its handshake completed", { "error": e.to_string() });
                return;
            }
        };

        let node = conn.remote_id();
        drop(slot);

        if conn.alpn() == AddressObserver::ALPN {
            if let Err(e) = crate::observe::ObserveResponder::answer(&conn, observed).await {
                curia::debug!("an address observation failed", { "node": node.to_string(), "error": e.to_string() });
            }
            return;
        }

        if let Err(e) = self.serve(&conn, node).await {
            curia::debug!("an enrollment connection ended", { "node": node.to_string(), "error": e.to_string() });
        }
    }

    async fn serve(&self, conn: &Connection, node: PublicKey) -> Result<(), EnrollError> {
        loop {
            let (mut send, mut recv) = conn
                .accept_bi()
                .await
                .map_err(|e| EnrollError::Transport(e.to_string()))?;

            let frame = Self::read_frame(&mut recv).await?;
            self.dispatch(node, frame, &mut send).await?;
        }
    }

    async fn dispatch(
        &self,
        node: PublicKey,
        frame: EnrollFrame,
        send: &mut SendStream,
    ) -> Result<(), EnrollError> {
        match frame {
            EnrollFrame::Hello { versions } => {
                match EnrollVersion::negotiate(EnrollVersion::SUPPORTED, &versions) {
                    Some(version) => Self::write(send, &EnrollFrame::Ready { version }).await,
                    None => Self::refuse(send, EnrollRefuseReason::NoCommonVersion).await,
                }
            }
            EnrollFrame::Enroll { token } => {
                match self.registry.redeem(&token, &node.to_string()).await {
                    Ok(name) => {
                        let name = self.zone.address_fqdn(&name);
                        Self::write(send, &EnrollFrame::Assigned { name }).await
                    }
                    Err(e) => {
                        curia::info!("refusing an enrollment", { "node": node.to_string(), "reason": e.to_string() });
                        Self::refuse(send, e.refuse_reason()).await
                    }
                }
            }
            EnrollFrame::PublishTxt { name, value } => self.publish_txt(node, name, value, send).await,
            EnrollFrame::CheckIn { address, port } => self.check_in(node, address, port, send).await,
            EnrollFrame::Challenge { .. }
            | EnrollFrame::ChallengeReply { .. }
            | EnrollFrame::DeclareAddress { .. } => {
                Self::refuse(send, EnrollRefuseReason::Internal).await
            }
            _ => Err(EnrollError::Unexpected {
                expected: "Hello, Enroll, PublishTxt or CheckIn",
            }),
        }
    }

    async fn publish_txt(
        &self,
        node: PublicKey,
        name: String,
        value: String,
        send: &mut SendStream,
    ) -> Result<(), EnrollError> {
        let Some(name) = self.zone.label_of(&name) else {
            return Self::refuse(send, EnrollRefuseReason::NameNotOwned).await;
        };

        match self.registry.name_for(&node.to_string()).await {
            Ok(Some(owned)) if owned == name => {
                // Renewals are exempt at the authority, so only a first issuance draws on the ceiling.
                let is_renewal = self.budget.has_issued(&name).await.unwrap_or(false);
                if !is_renewal && !self.budget.admits_new_issuance().await.unwrap_or(false) {
                    curia::warn!("refusing a first issuance: the weekly certificate budget is spent", { "name": name.clone() });
                    return Self::refuse(send, EnrollRefuseReason::Internal).await;
                }

                let cutoff = Self::now() - Self::CHALLENGE_TTL_SECONDS;
                self.zone.prune_challenges_before(cutoff).await?;
                if self.zone.open_challenges(&name).await? >= Self::MAX_OPEN_CHALLENGES {
                    curia::warn!("refusing a challenge: too many are already open for this name", { "name": name.clone(), "node": node.to_string() });
                    return Self::refuse(send, EnrollRefuseReason::Internal).await;
                }

                self.zone.publish_txt(&name, &value).await?;
                let _ = self.budget.record(&name, is_renewal).await;
                Self::write(send, &EnrollFrame::TxtPublished).await
            }
            Ok(_) => Self::refuse(send, EnrollRefuseReason::NameNotOwned).await,
            Err(e) => Self::refuse(send, e.refuse_reason()).await,
        }
    }

    async fn check_in(
        &self,
        node: PublicKey,
        address: Option<String>,
        port: u16,
        send: &mut SendStream,
    ) -> Result<(), EnrollError> {
        let address = match address.map(|a| a.parse::<IpAddr>()).transpose() {
            Ok(address) => address,
            Err(_) => {
                curia::info!("refusing a check-in whose address is not an IP address", { "node": node.to_string() });
                return Self::refuse(send, EnrollRefuseReason::Internal).await;
            }
        };

        let node_id = node.to_string();
        let record = match self.registry.record_check_in(&node_id, address, port).await {
            Ok(record) => record,
            Err(e) => return Self::refuse(send, e.refuse_reason()).await,
        };
        let reply = EnrollFrame::CheckedIn {
            nonce: record.nonce.into_bytes(),
        };

        match (record.state, address) {
            (RegistrationState::Revoked, _) => {
                // Replied first so the server holds the nonce before any probe arrives.
                Self::write(send, &reply).await?;
                self.evaluate_revoked(&node_id).await;
                return Ok(());
            }
            (RegistrationState::Active, address) => {
                if let Err(e) = self.sync_address(&record.name, address, record.address_changed).await {
                    curia::error!("updating a node's address record failed", { "node": node_id.clone(), "name": record.name.clone(), "error": e.to_string() });
                }
            }
            _ => {}
        }

        Self::write(send, &reply).await
    }

    // The nonce is already stored, so a DNS fault must not cost the node its reply.
    async fn sync_address(
        &self,
        name: &str,
        address: Option<IpAddr>,
        changed: bool,
    ) -> Result<(), crate::dns::DnsError> {
        match address {
            Some(ip) => {
                if changed || !self.zone.has_address_record(name).await? {
                    self.zone.publish_address(name, ip).await?;
                }
                Ok(())
            }
            None => self.zone.withdraw_address(name).await,
        }
    }

    async fn evaluate_revoked(&self, node_id: &str) {
        let claimed = match self
            .registry
            .claim_recovery_evaluation(node_id, Self::RECOVERY_COOLDOWN_SECONDS)
            .await
        {
            Ok(claimed) => claimed,
            Err(e) => {
                curia::warn!("claiming a recovery evaluation failed", { "node": node_id.to_string(), "error": e.to_string() });
                return;
            }
        };
        if claimed && let Err(e) = self.scheduler.evaluate_node(node_id).await {
            curia::warn!("evaluating a revoked registration on check-in failed", { "node": node_id.to_string(), "error": e.to_string() });
        }
    }

    fn now() -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or_default()
    }

    async fn refuse(send: &mut SendStream, reason: EnrollRefuseReason) -> Result<(), EnrollError> {
        Self::write(send, &EnrollFrame::Refuse { reason }).await
    }

    async fn write(send: &mut SendStream, frame: &EnrollFrame) -> Result<(), EnrollError> {
        send.write_all(&Framing::encode(frame)?)
            .await
            .map_err(|e| EnrollError::Transport(e.to_string()))?;
        send.finish()
            .map_err(|e| EnrollError::Transport(e.to_string()))?;
        Ok(())
    }

    async fn read_frame(recv: &mut RecvStream) -> Result<EnrollFrame, EnrollError> {
        let mut header = [0u8; Framing::HEADER_LEN];
        recv.read_exact(&mut header)
            .await
            .map_err(|e| EnrollError::Transport(e.to_string()))?;
        let len = Framing::payload_len(&header)?;
        let mut payload = vec![0u8; len];
        recv.read_exact(&mut payload)
            .await
            .map_err(|e| EnrollError::Transport(e.to_string()))?;
        Ok(Framing::decode(&payload)?)
    }
}
