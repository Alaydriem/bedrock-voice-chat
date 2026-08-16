// Why a `peer` block in `config.hcl` could not be turned into a grant.
//
// Every variant names the block that caused it, because an operator reading this
// at startup has a file full of blocks and no other way to tell which one is
// wrong.
#[derive(Debug, thiserror::Error)]
pub enum GrantConfigError {
    #[error("peer block {label:?} has an unreadable peerlink: {reason}")]
    PeerLink { label: String, reason: String },

    #[error("peer block {label:?} names an unknown capability {value:?}")]
    Capability { label: String, value: String },

    #[error("peer blocks {first:?} and {second:?} name the same peer")]
    DuplicateNode { first: String, second: String },
}
