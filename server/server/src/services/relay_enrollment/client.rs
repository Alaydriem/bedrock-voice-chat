use std::sync::Arc;
use std::time::Duration;

use bvc_relay::node::NodeIdentity;
use bvc_relay::peer::PeerEndpoint;
use common::structs::relay::enroll::{EnrollFrame, EnrollVersion, PlayPreflightOutcome};
use common::structs::relay::wire::Framing;
use iroh::EndpointAddr;
use iroh::endpoint::{Connection, RecvStream, SendStream};

use super::error::EnrollmentError;

pub struct RelayEnrollmentClient {
    registry: EndpointAddr,
    endpoint: PeerEndpoint,
}

impl RelayEnrollmentClient {
    pub const ALPN: &'static [u8] = b"bvc-enroll/1";

    // How long the registry may take to probe this server and report. It retries the probe
    // at intervals and each attempt can run to its own timeout.
    pub const PREFLIGHT_RESULT_TIMEOUT: Duration = Duration::from_secs(45);

    // The first enrollment version that carries the play frames.
    const PLAY_VERSION: EnrollVersion = EnrollVersion(3);

    pub async fn new(
        identity: &NodeIdentity,
        registry: EndpointAddr,
        port: Option<u16>,
    ) -> Result<Arc<Self>, EnrollmentError> {
        let endpoint = PeerEndpoint::bind_with_alpns(identity, port, vec![Self::ALPN.to_vec()])
            .await
            .map_err(|e| EnrollmentError::Connect(e.to_string()))?;

        Ok(Arc::new(Self { registry, endpoint }))
    }

    pub async fn enroll(&self, token: &str) -> Result<String, EnrollmentError> {
        match self
            .exchange(&EnrollFrame::Enroll {
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
            .exchange(&EnrollFrame::PublishTxt {
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

    pub async fn check_in(
        &self,
        address: Option<&str>,
        port: u16,
    ) -> Result<String, EnrollmentError> {
        match self
            .exchange(&EnrollFrame::CheckIn {
                address: address.map(str::to_string),
                port,
            })
            .await?
        {
            EnrollFrame::CheckedIn { nonce } => Ok(String::from_utf8_lossy(&nonce).into_owned()),
            EnrollFrame::Refuse { reason } => Err(EnrollmentError::refused(reason)),
            _ => Err(EnrollmentError::Unexpected {
                expected: "CheckedIn",
            }),
        }
    }

    /// Asks the registry for a play-zone name, and returns the fully qualified domain name.
    ///
    /// Idempotent on the registry's side: a node that already holds a name is given the same
    /// one back.
    pub async fn claim_play_name(&self) -> Result<String, EnrollmentError> {
        match self
            .exchange_at_least(&EnrollFrame::PlayClaim, Self::PLAY_VERSION)
            .await?
        {
            EnrollFrame::PlayAssigned { name } => Ok(name),
            EnrollFrame::Refuse { reason } => Err(EnrollmentError::refused(reason)),
            _ => Err(EnrollmentError::Unexpected {
                expected: "PlayAssigned",
            }),
        }
    }

    /// Sends a certificate request for the play name this node holds and returns the signed
    /// chain as PEM, leaf first.
    pub async fn request_play_certificate(
        &self,
        csr_der: Vec<u8>,
    ) -> Result<String, EnrollmentError> {
        match self
            .exchange_at_least(&EnrollFrame::PlayCertificate { csr_der }, Self::PLAY_VERSION)
            .await?
        {
            EnrollFrame::PlayIssued { chain_pem } => Ok(chain_pem),
            EnrollFrame::Refuse { reason } => Err(EnrollmentError::refused(reason)),
            _ => Err(EnrollmentError::Unexpected {
                expected: "PlayIssued",
            }),
        }
    }

    /// Asks the registry whether the internet can reach `port` on `address`, or on the
    /// address it sees this server connect from when `address` is `None`.
    ///
    /// The registry replies with a nonce when it will probe, and `on_nonce` receives it so
    /// the caller can serve it before the probe arrives. The outcome follows on the same
    /// stream. A registry that refuses before probing answers with the outcome alone.
    pub async fn play_preflight<F: FnOnce(String) + Send>(
        &self,
        address: Option<&str>,
        port: u16,
        on_nonce: F,
    ) -> Result<PlayPreflightOutcome, EnrollmentError> {
        let conn = self
            .endpoint
            .endpoint()
            .connect(self.registry.clone(), Self::ALPN)
            .await
            .map_err(|e| EnrollmentError::Connect(e.to_string()))?;

        let frame = EnrollFrame::PlayPreflight {
            address: address.map(str::to_string),
            port,
        };
        let result = Self::preflight_exchange(&conn, &frame, on_nonce).await;
        conn.close(0u32.into(), b"done");
        result
    }

    pub async fn close(&self) {
        self.endpoint.close().await;
    }

    async fn preflight_exchange<F: FnOnce(String) + Send>(
        conn: &Connection,
        frame: &EnrollFrame,
        on_nonce: F,
    ) -> Result<PlayPreflightOutcome, EnrollmentError> {
        Self::negotiate(conn, Some(Self::PLAY_VERSION)).await?;

        let (mut send, mut recv) = conn
            .open_bi()
            .await
            .map_err(|e| EnrollmentError::Transport(e.to_string()))?;
        Self::write(&mut send, frame).await?;

        match Self::read_frame(&mut recv).await? {
            EnrollFrame::PlayPreflightResult { outcome } => Ok(outcome),
            EnrollFrame::Refuse { reason } => Err(EnrollmentError::refused(reason)),
            EnrollFrame::PlayPreflightNonce { nonce } => {
                on_nonce(nonce);
                let next = tokio::time::timeout(
                    Self::PREFLIGHT_RESULT_TIMEOUT,
                    Self::read_frame(&mut recv),
                )
                .await
                .map_err(|_| {
                    EnrollmentError::Transport(
                        "timed out waiting for the preflight result".to_string(),
                    )
                })??;
                match next {
                    EnrollFrame::PlayPreflightResult { outcome } => Ok(outcome),
                    EnrollFrame::Refuse { reason } => Err(EnrollmentError::refused(reason)),
                    _ => Err(EnrollmentError::Unexpected {
                        expected: "PlayPreflightResult",
                    }),
                }
            }
            _ => Err(EnrollmentError::Unexpected {
                expected: "PlayPreflightNonce or PlayPreflightResult",
            }),
        }
    }

    async fn exchange(&self, frame: &EnrollFrame) -> Result<EnrollFrame, EnrollmentError> {
        self.exchange_with(frame, None).await
    }

    // The registry refuses play frames on a connection that negotiated an older version,
    // and a registry that predates them cannot decode the frame at all. The version it
    // answers `Hello` with is therefore checked before a play frame is sent.
    async fn exchange_at_least(
        &self,
        frame: &EnrollFrame,
        minimum: EnrollVersion,
    ) -> Result<EnrollFrame, EnrollmentError> {
        self.exchange_with(frame, Some(minimum)).await
    }

    async fn exchange_with(
        &self,
        frame: &EnrollFrame,
        minimum: Option<EnrollVersion>,
    ) -> Result<EnrollFrame, EnrollmentError> {
        let conn = self
            .endpoint
            .endpoint()
            .connect(self.registry.clone(), Self::ALPN)
            .await
            .map_err(|e| EnrollmentError::Connect(e.to_string()))?;

        let result = Self::negotiate_and_request(&conn, frame, minimum).await;
        conn.close(0u32.into(), b"done");
        result
    }

    async fn negotiate_and_request(
        conn: &Connection,
        frame: &EnrollFrame,
        minimum: Option<EnrollVersion>,
    ) -> Result<EnrollFrame, EnrollmentError> {
        Self::negotiate(conn, minimum).await?;
        Self::request(conn, frame).await
    }

    async fn negotiate(
        conn: &Connection,
        minimum: Option<EnrollVersion>,
    ) -> Result<(), EnrollmentError> {
        let hello = EnrollFrame::Hello {
            versions: EnrollVersion::SUPPORTED.to_vec(),
        };
        match Self::request(conn, &hello).await? {
            EnrollFrame::Ready { version } => {
                if let Some(minimum) = minimum
                    && version < minimum
                {
                    return Err(EnrollmentError::PlayNotOffered { version: version.0 });
                }
                Ok(())
            }
            EnrollFrame::Refuse { .. } => Err(EnrollmentError::NoCommonVersion),
            _ => Err(EnrollmentError::Unexpected { expected: "Ready" }),
        }
    }

    async fn request(
        conn: &Connection,
        frame: &EnrollFrame,
    ) -> Result<EnrollFrame, EnrollmentError> {
        let (mut send, mut recv) = conn
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
