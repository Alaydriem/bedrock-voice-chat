pub mod addon_mode_resolver;
pub mod advertised_version_resolver;
pub mod auth;
pub mod connector;
pub mod gating;
pub mod protocol_version_catalog;
pub mod session_name;
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

/// Marks a command failure the app must answer with a sign-in rather than a retry.
///
/// A sentinel rather than prose because the front end matches on it. Commands whose error type
/// is `String` have nowhere else to carry the distinction, and the typed outcome already has a
/// home on `bedrock_force_refresh`.
pub const REAUTH_REQUIRED: &str = "REAUTH_REQUIRED";

pub use addon_mode_resolver::AddonModeResolver;
pub use advertised_version_resolver::AdvertisedVersionResolver;
pub use auth::{BedrockAuthService, BedrockKeyringService};
pub use connector::{BedrockConnector, ProxyConnectRequest, RealmConnectRequest};
pub use protocol_version_catalog::ProtocolVersionCatalog;
pub use session_name::SessionName;
pub use gating::ProtocolGatingService;
pub(crate) use proxy::ProxyDeps;
pub use proxy::{
    BedrockChatChannel, BedrockConnectErrorChannel, BedrockEventEmitter, BedrockPlayerStateCache,
    BedrockProxyManager, ChatCodec, ChatInjector, ChatLine, DiscNbt, JukeboxBeaconCache,
    JukeboxEjectInjector, MinecraftTranslation, PendingEject, PendingQueryState,
    QueryStateInjector,
};
pub use state::BedrockState;
pub use target::{BedrockTargetService, ResolvedAddress, ResolvedTarget, SavedProxyEntry};
pub use transfer_keep_alive::TransferKeepAlive;
