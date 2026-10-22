use std::collections::HashMap;

use super::SframeCipher;

/// The keys and roster behind a [`super::SealGate`] while this client holds a protected group.
pub struct GateState {
    pub cipher: SframeCipher,
    /// MLS leaf → canonical identity, from the group's own state, so a frame is attributed to the
    /// member whose key sealed it rather than to whatever the server stamped on it.
    pub roster: HashMap<u32, String>,
}
