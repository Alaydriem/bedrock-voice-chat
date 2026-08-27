use std::sync::Arc;

use bvc_relay::node::NodeIdentity;
use bvc_relay::peer::{AddressObserver, AdmissionControl, PeerEndpoint};
use common::curia;
use common::structs::relay::enroll::{EnrollFrame, EnrollRefuseReason, EnrollVersion};
use common::structs::relay::wire::Framing;
use iroh::endpoint::{Connection, RecvStream, SendStream};
use iroh::PublicKey;

use crate::budget::WeeklyBudget;
use crate::dns::ZoneWriter;
use crate::registry::RegistryService;

use crate::enroll::{EnrollError, EnrollSession, EnrollSessions};

// The registry's iroh endpoint.
//
// One endpoint, two protocols, dispatched by ALPN. Enrollment is for entitled members
// and is gated by a single-use token; address observation is for every server, member
// or not, because peering is not a paid feature. They share a socket and nothing else.
//
// Neither rides the peer wire: `Handshake::accept` refuses any node not declared in a
// peer block, and neither an enrolling server nor one asking for its own address is
// anybody's declared peer.
pub struct RegistryEndpoint {
    endpoint: PeerEndpoint,
    registry: Arc<RegistryService>,
    zone: Arc<ZoneWriter>,
    budget: Arc<WeeklyBudget>,
    sessions: Arc<EnrollSessions>,
    admission: AdmissionControl,
}

impl RegistryEndpoint {
    pub const ALPN: &'static [u8] = b"bvc-enroll/1";

    // Connections still completing their handshake, not sessions already established.
    // The slot is released the moment a connection yields an authenticated node key,
    // so this bounds the anonymous window rather than the number of servers the relay
    // can serve — which must not be capped at all.
    //
    // Sized above the peer plane's 64, which is for declared peers: a burst of
    // enrollments must not starve voice peering, nor the reverse.
    pub const MAX_UNAUTHORIZED: usize = 256;

    pub async fn bind(
        identity: &NodeIdentity,
        registry: Arc<RegistryService>,
        zone: Arc<ZoneWriter>,
        budget: Arc<WeeklyBudget>,
        port: Option<u16>,
    ) -> Result<Arc<Self>, EnrollError> {
        let endpoint = PeerEndpoint::bind_with_alpns(
            identity,
            port,
            vec![
                Self::ALPN.to_vec(),
                AddressObserver::ALPN.to_vec(),
            ],
        )
        .await
        .map_err(|e| EnrollError::Bind(e.to_string()))?;

        Ok(Arc::new(Self {
            endpoint,
            registry,
            zone,
            budget,
            sessions: EnrollSessions::new_shared(),
            admission: AdmissionControl::new(Self::MAX_UNAUTHORIZED),
        }))
    }

    pub fn sessions(&self) -> &Arc<EnrollSessions> {
        &self.sessions
    }

    pub fn node_id(&self) -> PublicKey {
        self.endpoint.node_id()
    }

    // Where a server reaches this relay. Published in operator documentation and
    // pasted into a server's `relay_registry` block, so it is minted from the live
    // endpoint rather than assembled from configuration.
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

        // Read before the connection is awaited. Iroh exposes the address a
        // connection arrived from on `Incoming` and not on `Connection`, so a
        // responder that waited would have nothing left to report.
        let observed = incoming.remote_addr();

        let conn = match incoming.await {
            Ok(conn) => conn,
            Err(e) => {
                curia::debug!(format!(
                    "a registry connection failed before its handshake completed: {e}"
                ));
                return;
            }
        };

        // The link's own TLS proves the key, so the connection is authenticated the
        // moment it exists. Releasing here is what keeps the cap a bound on
        // handshakes rather than on how many servers the relay can hold sessions
        // with.
        let node = conn.remote_id();
        drop(slot);

        // Address observation is a single exchange with no session and no state, so it
        // is answered and forgotten. Only enrollment holds a session open.
        if conn.alpn() == AddressObserver::ALPN {
            if let Err(e) = crate::observe::ObserveResponder::answer(&conn, observed).await {
                curia::debug!(format!("an address observation failed: {e}"), { "node": node.to_string() });
            }
            return;
        }

        self.sessions.insert(EnrollSession::new(conn.clone(), node));

        if let Err(e) = self.serve(&conn, node).await {
            curia::debug!(format!("enrollment session ended: {e}"), { "node": node.to_string() });
        }

        self.sessions.remove(&node);
    }

    // One stream per request. The session stays open between them, which is what the
    // daily challenge rides.
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
                    // Fully qualified on the wire. The label alone is not a hostname:
                    // a server would publish it as its own name, present it as a SAN,
                    // and ask the certificate authority to sign it — which fails at
                    // the order, because a bare label is not a domain.
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
            // The node may publish for its own name and no other. The link
            // authenticates the node cryptographically, so this is a lookup rather
            // than a credential check — and a tighter scope than a shared key could
            // express.
            EnrollFrame::PublishTxt { name, value } => {
                // The node was handed a fully qualified name, so that is what it sends
                // back. The label is what everything here is keyed by.
                let Some(name) = self.zone.label_of(&name) else {
                    return Self::refuse(send, EnrollRefuseReason::NameNotOwned).await;
                };

                match self.registry.name_for(&node.to_string()).await {
                    Ok(Some(owned)) if owned == name => {
                        // A name that has held a certificate before is renewing, and
                        // a renewal is exempt at the authority. Only a first issuance
                        // draws on the weekly ceiling, and one refused here is one
                        // the authority would have rejected — burning the order and
                        // delaying the operator further.
                        let is_renewal = self.budget.has_issued(&name).await.unwrap_or(false);
                        if !is_renewal
                            && !self.budget.admits_new_issuance().await.unwrap_or(false)
                        {
                            curia::warn!("refusing a first issuance: the weekly certificate budget is spent", { "name": name.clone() });
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
            // Recorded before it is published. The daily pass reads the stored column
            // to decide whether to bind the record to this node, so an address
            // published without being recorded is one nothing ever verifies.
            EnrollFrame::DeclareAddress { address } => {
                match self
                    .registry
                    .declare_address(&node.to_string(), &address)
                    .await
                {
                    Ok(name) => {
                        self.zone.publish_a(&name, &address).await?;
                        Self::write(send, &EnrollFrame::TxtPublished).await
                    }
                    Err(e) => Self::refuse(send, e.refuse_reason()).await,
                }
            }
            _ => Err(EnrollError::Unexpected {
                expected: "Hello, Enroll, PublishTxt or DeclareAddress",
            }),
        }
    }

    // Sent before the caller sees an error, so the far side learns why rather than
    // seeing a bare close it would read as a network fault and retry.
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
