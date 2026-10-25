use std::sync::Arc;

use bvc_relay::node::NodeIdentity;
use bvc_relay::peer::PeerEndpoint;
use common::structs::relay::enroll::{EnrollFrame, EnrollVersion};
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

    pub async fn close(&self) {
        self.endpoint.close().await;
    }

    async fn exchange(&self, frame: &EnrollFrame) -> Result<EnrollFrame, EnrollmentError> {
        let conn = self
            .endpoint
            .endpoint()
            .connect(self.registry.clone(), Self::ALPN)
            .await
            .map_err(|e| EnrollmentError::Connect(e.to_string()))?;

        let result = Self::negotiate_and_request(&conn, frame).await;
        conn.close(0u32.into(), b"done");
        result
    }

    async fn negotiate_and_request(
        conn: &Connection,
        frame: &EnrollFrame,
    ) -> Result<EnrollFrame, EnrollmentError> {
        let hello = EnrollFrame::Hello {
            versions: EnrollVersion::SUPPORTED.to_vec(),
        };
        match Self::request(conn, &hello).await? {
            EnrollFrame::Ready { .. } => {}
            EnrollFrame::Refuse { .. } => return Err(EnrollmentError::NoCommonVersion),
            _ => return Err(EnrollmentError::Unexpected { expected: "Ready" }),
        }

        Self::request(conn, frame).await
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
