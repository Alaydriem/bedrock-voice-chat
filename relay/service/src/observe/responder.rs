use bvc_relay::peer::AddressObserver;
use iroh::endpoint::{Connection, IncomingAddr};

use crate::enroll::EnrollError;

// The registry's half of address observation.
//
// A thin shell over `AddressObserver::reply_to`, which owns the wire because both
// halves have to agree on it. This exists so the dispatch site names a registry type
// rather than reaching into the transport crate for its error mapping.
//
// The observed address is passed in rather than read here: iroh exposes it on
// `Incoming`, before the handshake completes, and it is gone by the time there is a
// `Connection` to answer on.
pub struct ObserveResponder;

impl ObserveResponder {
    pub async fn answer(conn: &Connection, observed: IncomingAddr) -> Result<(), EnrollError> {
        AddressObserver::reply_to(conn, observed)
            .await
            .map_err(|e| EnrollError::Transport(e.to_string()))
    }
}
