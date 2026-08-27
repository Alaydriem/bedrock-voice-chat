use serde::{Deserialize, Serialize};

// The enrollment wire's version.
//
// Independent of `WireVersion`. The peer wire carries voice and evolves with the
// audio path; this one carries registration and evolves with the registry. Sharing
// a number would make a change to either force a negotiation failure on the other.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct EnrollVersion(pub u16);

impl EnrollVersion {
    // Every version this build can speak, ascending.
    pub const SUPPORTED: &'static [EnrollVersion] = &[EnrollVersion(1)];

    // The highest version both sides can speak, or `None` when they share none.
    pub fn negotiate(local: &[EnrollVersion], remote: &[EnrollVersion]) -> Option<EnrollVersion> {
        local.iter().filter(|v| remote.contains(v)).max().copied()
    }
}
