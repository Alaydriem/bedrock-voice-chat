/// Why a group could not be resolved from a name.
#[derive(Debug, thiserror::Error)]
pub enum GroupError {
    #[error("no group named {0}")]
    NotFound(String),
    // Refused rather than resolved. `Channel` carries no creation time, so there is no defensible
    // way to choose between two, and afterwards the two are indistinguishable to the operator —
    // they would be talking to whoever happened to be in the one that got picked.
    #[error("{count} groups are named {name}; rename one and try again")]
    Ambiguous { name: String, count: usize },
}
