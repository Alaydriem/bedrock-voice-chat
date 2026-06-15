pub mod audio;
pub mod background_task;
pub mod delivery;
pub mod discovery;
pub mod manager;
pub mod orchestrator;
pub mod peer;
pub mod presence;
pub mod relayed_packet;

#[cfg(test)]
mod e2e_test;

pub use manager::{RelayManager, RelayManagerConfig};

pub use audio::{
    AudioFileExistence, AudioPeerQuery, AudioPuller, AudioSource, DbAudioFileExistence, RelayAudioPuller,
    ResolvedAudio,
};
pub use background_task::{ActiveWorldsSource, FnActiveWorldsSource, RelayBackgroundTask};
pub use delivery::{BroadcastInjectDelivery, LinkEchoDelivery};
pub use discovery::{
    EndpointReachability, HttpEndpointReachability, RegisterNonceStore, RelayClient, RelayRegistry,
};
pub use orchestrator::{LocalInjectDelivery, PeerEchoDelivery, RelayOrchestrator};
pub use peer::{
    Caps, GatedPeerIngest, IDLE_TIMEOUT, PeerCertIssueError, PeerCertIssuer, PeerDialDriver,
    PeerDialer, PeerDirection, PeerLink, PeerLinkIngest, PeerLinkState, PeerManager, PeerRole,
    PeerTable, ProductionPeerDialDriver, RelayIngestSink, WebhookIngestSink,
};
pub use presence::{AlwaysProven, NeverProven, PresenceGate, PresenceProver, CHALLENGE_TTL};
pub use relayed_packet::{PacketOrigin, RelayedPacket};
