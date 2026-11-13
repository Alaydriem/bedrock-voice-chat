use common::structs::packet::QuicNetworkPacket;

mod cached_endpoint;
mod credential_fault;
mod position_socket;
mod position_socket_registry;
mod public_server_config_reader;
mod remote_image_fetcher;
mod server_endpoint_expiry;
mod server_endpoint_service;
mod stream;
mod transport_verdict;

use serde::{Deserialize, Serialize};

pub use credential_fault::CredentialFault;
pub(crate) use position_socket::PositionSocket;
pub(crate) use position_socket_registry::PositionSocketRegistry;
pub(crate) use public_server_config_reader::PublicServerConfigReader;
pub use remote_image_fetcher::RemoteImageFetcher;
pub use cached_endpoint::CachedEndpoint;
pub use server_endpoint_expiry::ServerEndpointExpiry;
pub use server_endpoint_service::ServerEndpointService;
pub(crate) use stream::ConnectFailure;
pub use stream::{AttemptResult, ConnectOutcome, FallbackReason, HealthPublisher};
pub(crate) use stream::NetworkStreamManager;
pub(crate) use stream::link::MtlsClientConfig;
pub use transport_verdict::TransportVerdict;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkPacket {
    pub data: QuicNetworkPacket,
}
