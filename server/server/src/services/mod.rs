//! Server services

#[cfg(feature = "bedrock")]
pub mod bedrock;
#[cfg(feature = "bedrock")]
pub use bedrock::{DnsService, TransferRelayService, TransferTargetCache};

pub mod acme;
pub mod audio_file_service;
pub mod audio_playback_service;
pub mod audio_stream_token_cache;
pub mod auth_code_service;
pub mod auth_service;
pub mod bedrock_event_service;
pub mod channel_membership_service;
pub mod client_action_service;
pub mod certificate_service;
pub mod gamerpic_decoder;
pub mod health_service;
pub mod meridian_service;
pub mod permission_service;
pub mod metrics_service;
pub mod player_identity_service;
pub mod player_registrar_service;

pub use audio_file_service::{AudioFileError, AudioFileService};
pub use audio_playback_service::{AudioPlaybackService, EjectScheduler};
pub use audio_stream_token_cache::AudioStreamTokenCache;
pub use auth_code_service::{AuthCodeError, AuthCodeService};
pub use auth_service::{AuthError, AuthService, CodeLoginError};
pub use bedrock_event_service::{BedrockEventRejection, BedrockEventService};
pub use channel_membership_service::ChannelMembershipService;
pub use client_action_service::ClientActionService;
pub use certificate_service::CertificateService;
pub use gamerpic_decoder::GamerpicDecoder;
pub use health_service::HealthService;
pub use meridian_service::MeridianService;
pub use permission_service::{PermissionService, PermissionServiceError};
pub use metrics_service::MetricsService;
pub use player_identity_service::PlayerIdentityService;
pub use player_registrar_service::{PlayerRegistrarService, RegisteredPlayersCache};
