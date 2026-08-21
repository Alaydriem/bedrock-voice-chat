use curia::Fields;
use serde::Serialize;

// Only a record carrying one of these becomes a Sentry Issue. Adding a variant
// is a review of this whole file, which is the point: error level alone must not
// promote a record to an Issue, or the July 2026 log storm returns at Issue
// prices.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Defect {
    AudioDeviceLost,
    AudioDeviceRebuildFailed,
    EncoderInitFailed,
    DecoderResetLoop,
    JitterBufferStarved,
    RecordingWalFailed,
    RecordingManifestFailed,
    QuicHandshakeFailed,
    TransportFellBack,
    CertificateRejected,
    KeyringWriteFailed,
    ChannelJoinFailed,
    PositionFeedStalled,
}

// NAMES, as_str and from_index must stay in the same order. The vocabulary's
// tag variant set is NAMES, and its tests catch either half drifting.
const ALL: &[Defect] = &[
    Defect::AudioDeviceLost,
    Defect::AudioDeviceRebuildFailed,
    Defect::EncoderInitFailed,
    Defect::DecoderResetLoop,
    Defect::JitterBufferStarved,
    Defect::RecordingWalFailed,
    Defect::RecordingManifestFailed,
    Defect::QuicHandshakeFailed,
    Defect::TransportFellBack,
    Defect::CertificateRejected,
    Defect::KeyringWriteFailed,
    Defect::ChannelJoinFailed,
    Defect::PositionFeedStalled,
];

impl Defect {
    pub const NAMES: &'static [&'static str] = &[
        "AudioDeviceLost",
        "AudioDeviceRebuildFailed",
        "EncoderInitFailed",
        "DecoderResetLoop",
        "JitterBufferStarved",
        "RecordingWalFailed",
        "RecordingManifestFailed",
        "QuicHandshakeFailed",
        "TransportFellBack",
        "CertificateRejected",
        "KeyringWriteFailed",
        "ChannelJoinFailed",
        "PositionFeedStalled",
    ];

    pub fn all() -> &'static [Defect] {
        ALL
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::AudioDeviceLost => "AudioDeviceLost",
            Self::AudioDeviceRebuildFailed => "AudioDeviceRebuildFailed",
            Self::EncoderInitFailed => "EncoderInitFailed",
            Self::DecoderResetLoop => "DecoderResetLoop",
            Self::JitterBufferStarved => "JitterBufferStarved",
            Self::RecordingWalFailed => "RecordingWalFailed",
            Self::RecordingManifestFailed => "RecordingManifestFailed",
            Self::QuicHandshakeFailed => "QuicHandshakeFailed",
            Self::TransportFellBack => "TransportFellBack",
            Self::CertificateRejected => "CertificateRejected",
            Self::KeyringWriteFailed => "KeyringWriteFailed",
            Self::ChannelJoinFailed => "ChannelJoinFailed",
            Self::PositionFeedStalled => "PositionFeedStalled",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        ALL.iter().find(|d| d.as_str() == value).copied()
    }

    // An unrecognised value yields None, so a typo degrades to an attribute
    // rather than creating an untracked Issue.
    pub fn from_fields(fields: &Fields) -> Option<Self> {
        fields
            .get("defect")
            .and_then(|v| v.as_str())
            .and_then(Self::parse)
    }
}
