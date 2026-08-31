use std::sync::Arc;

use bvc_relay::node::NodeIdentity;
use bvc_relay::peer::PeerEndpoint;
use common::curia;
use common::structs::relay::enroll::{EnrollFrame, EnrollVersion};
use common::structs::relay::wire::Framing;
use iroh::endpoint::{Connection, RecvStream, SendStream};
use iroh::{EndpointAddr, SecretKey};

use super::error::EnrollmentError;
use super::nonce::CurrentNonce;

// This server's link to the relay registry.
//
// The session is held for the life of the process rather than dialled per exchange.
// The relay cannot dial in: the endpoint is built on iroh's `Minimal` preset, which
// has no discovery, so a bare node id resolves to nothing. Holding the connection
// this side opened is what lets the relay push a challenge to a server behind CGNAT
// with the same code that reaches one with a public address.
pub struct RelayEnrollmentClient {
    conn: Connection,
    endpoint: PeerEndpoint,
}

impl RelayEnrollmentClient {
    pub const ALPN: &'static [u8] = b"bvc-enroll/1";

    pub async fn connect(
        identity: &NodeIdentity,
        registry: EndpointAddr,
        port: Option<u16>,
    ) -> Result<Arc<Self>, EnrollmentError> {
        let endpoint =
            PeerEndpoint::bind_with_alpns(identity, port, vec![Self::ALPN.to_vec()])
                .await
                .map_err(|e| EnrollmentError::Connect(e.to_string()))?;

        let conn = endpoint
            .endpoint()
            .connect(registry, Self::ALPN)
            .await
            .map_err(|e| EnrollmentError::Connect(e.to_string()))?;

        let client = Arc::new(Self { conn, endpoint });
        client.negotiate().await?;
        Ok(client)
    }

    async fn negotiate(&self) -> Result<EnrollVersion, EnrollmentError> {
        match self
            .request(&EnrollFrame::Hello {
                versions: EnrollVersion::SUPPORTED.to_vec(),
            })
            .await?
        {
            EnrollFrame::Ready { version } => Ok(version),
            EnrollFrame::Refuse { .. } => Err(EnrollmentError::NoCommonVersion),
            _ => Err(EnrollmentError::Unexpected { expected: "Ready" }),
        }
    }

    pub async fn enroll(&self, token: &str) -> Result<String, EnrollmentError> {
        match self
            .request(&EnrollFrame::Enroll {
                token: token.to_string(),
            })
            .await?
        {
            EnrollFrame::Assigned { name } => Ok(name),
            EnrollFrame::Refuse { reason } => Err(EnrollmentError::refused(reason)),
            _ => Err(EnrollmentError::Unexpected {
                expected: "Assigned",
            }),
        }
    }

    pub async fn publish_txt(&self, name: &str, value: &str) -> Result<(), EnrollmentError> {
        match self
            .request(&EnrollFrame::PublishTxt {
                name: name.to_string(),
                value: value.to_string(),
            })
            .await?
        {
            EnrollFrame::TxtPublished => Ok(()),
            EnrollFrame::Refuse { reason } => Err(EnrollmentError::refused(reason)),
            _ => Err(EnrollmentError::Unexpected {
                expected: "TxtPublished",
            }),
        }
    }

    pub async fn declare_address(&self, address: &str) -> Result<(), EnrollmentError> {
        match self
            .request(&EnrollFrame::DeclareAddress {
                address: address.to_string(),
            })
            .await?
        {
            EnrollFrame::TxtPublished => Ok(()),
            EnrollFrame::Refuse { reason } => Err(EnrollmentError::refused(reason)),
            _ => Err(EnrollmentError::Unexpected {
                expected: "TxtPublished",
            }),
        }
    }

    // Answers challenges the relay pushes down the held session, and records the
    // nonce so the HTTP route can echo it.
    //
    // Ends when the session does. A dropped session is not fatal to the server: the
    // certificate on disk outlives it, and the next boot re-establishes one.
    pub fn spawn_challenge_responder(
        self: &Arc<Self>,
        secret: SecretKey,
        nonce: Arc<CurrentNonce>,
    ) {
        let client = Arc::clone(self);
        tokio::spawn(async move {
            loop {
                let (mut send, mut recv) = match client.conn.accept_bi().await {
                    Ok(pair) => pair,
                    Err(e) => {
                        curia::warn!(format!("the relay enrollment session ended: {e}"));
                        return;
                    }
                };

                let frame = match Self::read_frame(&mut recv).await {
                    Ok(frame) => frame,
                    Err(e) => {
                        curia::debug!(format!("a relay challenge could not be read: {e}"));
                        continue;
                    }
                };

                let EnrollFrame::Challenge { nonce: value } = frame else {
                    continue;
                };

                nonce.set(String::from_utf8_lossy(&value).to_string());

                let reply = EnrollFrame::ChallengeReply {
                    signature: secret.sign(&value).to_bytes().to_vec(),
                };
                if let Err(e) = Self::write(&mut send, &reply).await {
                    curia::debug!(format!("answering a relay challenge failed: {e}"));
                }
            }
        });
    }

    pub async fn close(&self) {
        self.endpoint.close().await;
    }

    async fn request(&self, frame: &EnrollFrame) -> Result<EnrollFrame, EnrollmentError> {
        let (mut send, mut recv) = self
            .conn
            .open_bi()
            .await
            .map_err(|e| EnrollmentError::Transport(e.to_string()))?;

        Self::write(&mut send, frame).await?;
        Self::read_frame(&mut recv).await
    }

    async fn write(send: &mut SendStream, frame: &EnrollFrame) -> Result<(), EnrollmentError> {
        send.write_all(&Framing::encode(frame)?)
            .await
            .map_err(|e| EnrollmentError::Transport(e.to_string()))?;
        send.finish()
            .map_err(|e| EnrollmentError::Transport(e.to_string()))?;
        Ok(())
    }

    async fn read_frame(recv: &mut RecvStream) -> Result<EnrollFrame, EnrollmentError> {
        let mut header = [0u8; Framing::HEADER_LEN];
        recv.read_exact(&mut header)
            .await
            .map_err(|e| EnrollmentError::Transport(e.to_string()))?;
        let len = Framing::payload_len(&header)?;
        let mut payload = vec![0u8; len];
        recv.read_exact(&mut payload)
            .await
            .map_err(|e| EnrollmentError::Transport(e.to_string()))?;
        Ok(Framing::decode(&payload)?)
    }
}
