pub mod advertised_version_resolver;
pub mod auth;
pub mod connector;
pub mod gating;
pub mod protocol_version_catalog;
pub mod proxy;
pub mod state;
pub mod transfer_keep_alive;

pub use advertised_version_resolver::AdvertisedVersionResolver;
pub use auth::{BedrockAuthService, BedrockKeyringService};
pub use connector::{BedrockConnector, ProxyConnectRequest, RealmConnectRequest};
pub use protocol_version_catalog::ProtocolVersionCatalog;
pub use gating::ProtocolGatingService;
pub(crate) use proxy::ProxyDeps;
pub use proxy::{
    AnnounceInjector, BedrockConnectErrorChannel, BedrockEventEmitter, BedrockPlayerStateCache,
    BedrockChatChannel, BedrockProxyManager, BvcpCodec, ChatCodec, ChatInjector, ChatLine,
    DiscNbt, JukeboxBeaconCache, MinecraftTranslation,
    JukeboxEjectInjector,
    PendingAnnounce, PendingEject, PendingInject, PendingQueryState, PresenceInjector,
    QueryStateInjector,
};
pub use state::BedrockState;
pub use transfer_keep_alive::TransferKeepAlive;
