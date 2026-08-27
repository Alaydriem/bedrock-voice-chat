use serde::{Deserialize, Serialize};

use super::refuse_reason::EnrollRefuseReason;
use super::version::EnrollVersion;

// Everything that travels on an enrollment stream.
//
// Append-only. Postcard encodes a variant as its index, so a variant inserted
// anywhere but the end shifts every later discriminant and silently mis-decodes
// frames from a peer built against a different order.
//
// Deliberately not part of `ControlFrame`: that enum is the voice peer wire, and
// mixing enrollment into it would couple the registry's release cadence to the
// audio path's.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EnrollFrame {
    Hello { versions: Vec<EnrollVersion> },
    Ready { version: EnrollVersion },
    Enroll { token: String },
    Assigned { name: String },
    PublishTxt { name: String, value: String },
    TxtPublished,
    Challenge { nonce: Vec<u8> },
    ChallengeReply { signature: Vec<u8> },
    DeclareAddress { address: String },
    Refuse { reason: EnrollRefuseReason },
}
