use serde::{Deserialize, Serialize};

// The peer wire's version.
//
// Deliberately not pinned to the client `protocol_version`, because the two decide
// compatibility in incompatible ways. `ProtocolCompatibility` demands exact
// major.minor equality and cannot express speaking more than one version; this
// negotiates the highest version both sides list. Pinning would impose exact
// equality here, so a client-protocol bump made for a reason that never touches
// peering would break every third-party bridge at once, with no version they could
// have advertised to stay compatible.
//
// The two are not fully independent: a voice frame carries `PlayerEnum`, which the
// client protocol also carries, so a change there moves both encodings while this
// number sits still. The golden-frame byte test is what catches that.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct WireVersion(pub u16);

impl WireVersion {
    // Every version this build can speak, ascending.
    pub const SUPPORTED: &'static [WireVersion] = &[WireVersion(1)];

    // The highest version both sides can speak, or `None` when they share none.
    //
    // `None` is a refusal, not a fallback: a link established on a version one
    // side does not understand fails later, in the middle of a call, rather than
    // at connect where it can be reported.
    pub fn negotiate(local: &[WireVersion], remote: &[WireVersion]) -> Option<WireVersion> {
        local.iter().filter(|v| remote.contains(v)).max().copied()
    }
}
