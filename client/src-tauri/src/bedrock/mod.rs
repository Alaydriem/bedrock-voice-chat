pub mod advertised_version_resolver;
pub mod auth;
pub mod connector;
pub mod gating;
pub mod protocol_version_catalog;
pub mod proxy;
pub mod state;
pub mod target;
pub mod transfer_keep_alive;

/// Raised whenever a target is listed or connected without Xbox Live authentication.
///
/// One string because both surfaces refuse for the same reason: a proxy needs this
/// authentication as much as a realm does, so a list that omitted only realms would name
/// worlds that cannot be connected.
pub const XBOX_AUTH_REQUIRED: &str = "Xbox Live authentication required. Please sign in first.";

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
pub use target::{BedrockTargetService, ResolvedAddress, ResolvedTarget, SavedProxyEntry};
pub use transfer_keep_alive::TransferKeepAlive;
