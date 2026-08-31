use bvc_relay::peer::{EnrolOutcome, Enrolment};

use crate::error::SdkError;

// What one enrolment attempt decided.
//
// Mirrors `bvc_relay::peer::EnrolOutcome` rather than re-exporting it, because uniffi
// exports the types it is given and `bvc-relay` must not grow a binding-generator
// dependency to satisfy this crate.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum SdkEnrolOutcome {
    Paired { worlds: Vec<String> },
    WrongCode,
    Expired,
    NotAuthorized,
    NoSharedWorld,
    NoCommonVersion,
    Unreachable,
}

impl From<EnrolOutcome> for SdkEnrolOutcome {
    fn from(outcome: EnrolOutcome) -> Self {
        match outcome {
            EnrolOutcome::Paired { worlds } => Self::Paired { worlds },
            EnrolOutcome::WrongCode => Self::WrongCode,
            EnrolOutcome::Expired => Self::Expired,
            EnrolOutcome::NotAuthorized => Self::NotAuthorized,
            EnrolOutcome::NoSharedWorld => Self::NoSharedWorld,
            EnrolOutcome::NoCommonVersion => Self::NoCommonVersion,
            EnrolOutcome::Unreachable => Self::Unreachable,
        }
    }
}

/// Redeems a pairing code against a BVC server, once.
///
/// A free function rather than a method on `BvcPeer`: enrolment happens before a session
/// can succeed, and modelling it as a method would require holding an object whose whole
/// purpose is to fail until this has run.
///
/// `node_dir` must be the directory the session will later use. The grant this writes is
/// pinned to that key, and a different one would pair a peer that never connects.
#[uniffi::export(async_runtime = "tokio")]
pub async fn bvc_enrol(
    node_dir: String,
    peerlink: String,
    worlds: Vec<String>,
    code: String,
) -> Result<SdkEnrolOutcome, SdkError> {
    Enrolment::redeem(&node_dir, &peerlink, worlds, code)
        .await
        .map(SdkEnrolOutcome::from)
        .map_err(|e| SdkError::Open {
            reason: e.to_string(),
        })
}
