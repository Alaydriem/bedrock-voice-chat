pub mod auth;
pub mod gating;
pub mod proxy;
pub mod state;
pub mod transfer_keep_alive;

pub use auth::{BedrockAuthService, BedrockKeyringService};
pub use gating::ProtocolGatingService;
pub use gating::RealmsConnectGatingService;
pub(crate) use proxy::ProxyDeps;
pub use proxy::{
    AnnounceInjector, BedrockConnectErrorChannel, BedrockEventEmitter, BedrockPlayerStateCache,
    BedrockProxyManager, BvcpCodec, DiscNbt, JukeboxBeaconCache, JukeboxEjectInjector,
    PendingAnnounce, PendingEject, PendingInject, PresenceInjector,
};
pub use state::BedrockState;
pub use transfer_keep_alive::TransferKeepAlive;
