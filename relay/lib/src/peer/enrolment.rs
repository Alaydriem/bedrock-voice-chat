use std::time::Duration;

use common::structs::relay::wire::control::RefuseReason;
use iroh::EndpointAddr;

use crate::node::{NodeIdentity, PeerTicket};

use super::endpoint::PeerEndpoint;
use super::error::PeerError;
use super::handshake::Handshake;

// What one enrolment attempt decided, in the terms an operator asked in.
//
// Distinct from `RefuseReason` because a bridge reports this to a person at a console: a
// code that was never minted and one already spent are the same refusal on the wire and
// different sentences to read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnrolOutcome {
    Paired { worlds: Vec<String> },
    WrongCode,
    Expired,
    NotAuthorized,
    NoSharedWorld,
    NoCommonVersion,
    Unreachable,
}

// Redeeming a pairing code, as one exchange that ends when it is answered.
//
// Deliberately not part of `PeerSession`: that dials in a loop and retries forever, which
// is right for a link and wrong for a code. A wrong code retried on a backoff schedule
// spends its attempt budget without anyone being told, and the operator who typed it is
// still waiting at a console for an answer.
pub struct Enrolment;

impl Enrolment {
    // One attempt, bounded. A person is waiting on this, and a dial that hangs is worse
    // than one that fails.
    const TIMEOUT: Duration = Duration::from_secs(15);

    /// Dials the peer named by `peerlink` and redeems `code` against it.
    ///
    /// Binds its own endpoint from `node_dir`, which is the same key the session will
    /// later present. That is what makes the grant this writes match the peer that
    /// connects afterwards.
    pub async fn redeem(
        node_dir: &str,
        peerlink: &str,
        worlds: Vec<String>,
        code: String,
    ) -> Result<EnrolOutcome, PeerError> {
        let addr: EndpointAddr =
            PeerTicket::parse(peerlink).map_err(|e| PeerError::Bind(format!("peerlink: {e}")))?;

        let identity =
            NodeIdentity::load_or_create(node_dir).map_err(|e| PeerError::Bind(e.to_string()))?;
        let endpoint = PeerEndpoint::bind(&identity).await?;

        let conn = match tokio::time::timeout(
            Self::TIMEOUT,
            endpoint.endpoint().connect(addr, PeerEndpoint::ALPN),
        )
        .await
        {
            Ok(Ok(conn)) => conn,
            // Both a timeout and a refused dial mean the same thing to the operator: the
            // address in the peer link did not answer.
            Ok(Err(_)) | Err(_) => return Ok(EnrolOutcome::Unreachable),
        };

        let outcome = match Handshake::enrol(&conn, worlds, code).await {
            Ok(enrolled) => EnrolOutcome::Paired {
                worlds: enrolled.worlds,
            },
            Err(PeerError::Refused(reason)) => match reason {
                RefuseReason::UnknownCode | RefuseReason::CodeSpent => EnrolOutcome::WrongCode,
                RefuseReason::CodeExpired => EnrolOutcome::Expired,
                RefuseReason::NotAuthorized => EnrolOutcome::NotAuthorized,
                RefuseReason::NoSharedWorld => EnrolOutcome::NoSharedWorld,
                RefuseReason::NoCommonVersion => EnrolOutcome::NoCommonVersion,
                RefuseReason::AtCapacity => EnrolOutcome::Unreachable,
            },
            Err(_) => EnrolOutcome::Unreachable,
        };

        // The endpoint exists only for this exchange. Closing it releases the socket
        // before the session that follows binds the same key.
        endpoint.close().await;

        Ok(outcome)
    }
}
