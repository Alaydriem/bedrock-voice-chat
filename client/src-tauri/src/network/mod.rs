use common::structs::packet::QuicNetworkPacket;

mod credential_fault;
mod stream;
mod transport_verdict;

use serde::{Deserialize, Serialize};

pub use credential_fault::CredentialFault;
pub(crate) use stream::ConnectFailure;
pub(crate) use stream::NetworkStreamManager;
pub use stream::HealthPublisher;
pub use transport_verdict::TransportVerdict;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkPacket {
    pub data: QuicNetworkPacket,
}
