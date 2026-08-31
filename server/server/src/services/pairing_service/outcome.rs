use common::structs::relay::Capability;

// What a redemption decided.
//
// `AlreadyPaired` is distinct from `Paired` because the two spend different things: a node
// that already holds a grant is answered from it, and the code it presented stays live for
// whoever it was actually minted for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RedeemOutcome {
    Paired {
        // The name the code was minted under, so the caller's in-memory grant is labelled
        // the same as the row and `relay peers` shows one name.
        label: String,
        worlds: Vec<String>,
        capabilities: Vec<Capability>,
    },
    AlreadyPaired {
        label: String,
        worlds: Vec<String>,
        capabilities: Vec<Capability>,
    },
    Unknown,
    Spent,
    Expired,
}
