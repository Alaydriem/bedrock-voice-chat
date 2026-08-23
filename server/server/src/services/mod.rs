//! Server services

#[cfg(feature = "bedrock")]
pub mod bedrock;
#[cfg(feature = "bedrock")]
pub use bedrock::{TransferRelayService, TransferTargetCache};

pub mod acme;
pub mod admin_user_service;
pub mod audio_file_service;
pub mod audio_playback_service;
pub mod audio_stream_token_cache;
pub mod auth_code_service;
pub mod auth_service;
pub mod bedrock_event_service;
pub mod chat_service;
pub mod channel_membership_service;
pub mod client_action_service;
pub mod certificate_revocation_service;
pub mod certificate_service;
pub mod session_authorization_service;
pub mod gamerpic_decoder;
pub mod health_service;
pub mod meridian_service;
pub mod permission_service;
pub mod metrics_service;
pub mod player_identity_service;
pub mod player_registrar_service;
pub mod position_feed;
pub mod position_service;

pub use admin_user_service::AdminUserService;
pub use audio_file_service::{AudioFileError, AudioFileService};
pub use audio_playback_service::{AudioPlaybackService, EjectScheduler};
pub(crate) use audio_playback_service::SpeakerEntry;
pub use audio_stream_token_cache::AudioStreamTokenCache;
pub use auth_code_service::{AuthCodeError, AuthCodeService};
pub use auth_service::{AuthError, AuthService, CodeLoginError};
pub use bedrock_event_service::{BedrockEventRejection, BedrockEventService};
pub use channel_membership_service::ChannelMembershipService;
pub use chat_service::{ChatService, ChatSink, QuicChatSink};
pub use client_action_service::ClientActionService;
pub use certificate_revocation_service::CertificateRevocationService;
pub use certificate_service::CertificateService;
pub use gamerpic_decoder::GamerpicDecoder;
pub use health_service::HealthService;
pub use meridian_service::MeridianService;
pub use permission_service::{PermissionService, PermissionServiceError};
pub use metrics_service::MetricsService;
pub use metrics_service::host_capability::HostCapability;
pub use player_identity_service::PlayerIdentityService;
pub use player_registrar_service::{PlayerRegistrarService, RegisteredPlayersCache};
pub use position_feed::{GridCell, PositionFeedService, WorldIndex};
pub use position_service::{FAR_TIER_MAX, PositionService};

pub use session_authorization_service::{SessionAuthorizationService, SessionRejection};