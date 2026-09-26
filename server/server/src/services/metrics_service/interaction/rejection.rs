use common::errors::communication::{CommunicationError, GameError, MinecraftCommunicationError};

/// Why `route_audio_frame` withheld a frame: from every recipient, or from one.
///
/// The first two are whole-frame; every other variant is per recipient. Self-echo (the
/// sender appearing in its own recipient snapshot) is not listed, because it happens on every
/// frame by construction and counting it would swamp the rest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RouteRejection {
    SenderUnknown,
    NotAudio,
    SpeakerUnresolved,
    RecipientUnpositioned,
    GameMismatch,
    OutOfRange,
    WorldMismatch,
    DimensionMismatch,
    SpectatorInaudible,
    NonSpatialOutsideChannel,
    SerializeFailed,
    SequenceExhausted,
    RecipientQueueFull,
    RecipientClosed,
}

impl RouteRejection {
    pub const ALL: [RouteRejection; 14] = [
        Self::SenderUnknown,
        Self::NotAudio,
        Self::SpeakerUnresolved,
        Self::RecipientUnpositioned,
        Self::GameMismatch,
        Self::OutOfRange,
        Self::WorldMismatch,
        Self::DimensionMismatch,
        Self::SpectatorInaudible,
        Self::NonSpatialOutsideChannel,
        Self::SerializeFailed,
        Self::SequenceExhausted,
        Self::RecipientQueueFull,
        Self::RecipientClosed,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            Self::SenderUnknown => "sender_unknown",
            Self::NotAudio => "not_audio",
            Self::SpeakerUnresolved => "speaker_unresolved",
            Self::RecipientUnpositioned => "recipient_unpositioned",
            Self::GameMismatch => "game_mismatch",
            Self::OutOfRange => "out_of_range",
            Self::WorldMismatch => "world_mismatch",
            Self::DimensionMismatch => "dimension_mismatch",
            Self::SpectatorInaudible => "spectator_inaudible",
            Self::NonSpatialOutsideChannel => "non_spatial_outside_channel",
            Self::SerializeFailed => "serialize_failed",
            Self::SequenceExhausted => "sequence_exhausted",
            Self::RecipientQueueFull => "recipient_queue_full",
            Self::RecipientClosed => "recipient_closed",
        }
    }

    // Position in `ALL`, for the fixed-size arrays behind `RouteRejectionCounts` and the
    // resolved metric handles. A variant missing from `ALL` is a programming error and fails loudly.
    pub fn index(&self) -> usize {
        Self::ALL
            .iter()
            .position(|r| r == self)
            .expect("every RouteRejection variant is listed in ALL")
    }

    /// The proximity gate folds four distinct refusals into one error; this splits them back.
    pub fn from_communication_error(error: &CommunicationError) -> Self {
        match error {
            CommunicationError::OutOfRange { .. } => Self::OutOfRange,
            CommunicationError::GameMismatch { .. } => Self::GameMismatch,
            CommunicationError::Game(GameError::Minecraft(inner)) => match inner {
                MinecraftCommunicationError::WorldMismatch { .. } => Self::WorldMismatch,
                MinecraftCommunicationError::DimensionMismatch { .. } => Self::DimensionMismatch,
                MinecraftCommunicationError::SpectatorInaudible => Self::SpectatorInaudible,
            },
            CommunicationError::Game(GameError::Generic(_)) => Self::GameMismatch,
        }
    }
}
