use common::errors::communication::{CommunicationError, GameError, MinecraftCommunicationError};

/// Why `route_audio_frame` withheld a frame: from every recipient, or from one.
///
/// `SenderUnknown`, `NotAudio`, `UnsealedInProtectedChannel` and `GroupClaimUnverified` are
/// whole-frame; every other variant is per recipient. Self-echo (the
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
    /// Plaintext audio from a protected group's member, refused so it cannot reach anyone.
    UnsealedInProtectedChannel,
    /// A sealed frame offered to a recipient outside the sender's group.
    SealedOutsideChannel,
    /// Speech from a members-only group offered to a recipient outside it.
    MembersOnlyOutsideChannel,
    /// A frame claiming a linked group its speaker is not a member of. Whole-frame: it is
    /// neither delivered to the group nor routed by proximity.
    GroupClaimUnverified,
}

impl RouteRejection {
    pub const ALL: [RouteRejection; 18] = [
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
        Self::UnsealedInProtectedChannel,
        Self::SealedOutsideChannel,
        Self::MembersOnlyOutsideChannel,
        Self::GroupClaimUnverified,
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
            Self::UnsealedInProtectedChannel => "unsealed_in_protected_channel",
            Self::SealedOutsideChannel => "sealed_outside_channel",
            Self::MembersOnlyOutsideChannel => "members_only_outside_channel",
            Self::GroupClaimUnverified => "group_claim_unverified",
        }
    }

    // Position in `ALL`, for the fixed-size arrays behind `RouteRejectionCounts` and the
    // resolved metric handles. The discriminant, because this runs per rejected recipient per
    // frame; it holds only while `ALL` lists the variants in declaration order.
    pub fn index(&self) -> usize {
        *self as usize
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
