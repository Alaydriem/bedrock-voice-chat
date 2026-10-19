use super::LostReason;

/// What the key service tells the rest of the app. None of these carries key material.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProtectedGroupEvent {
    Joined { epoch: u64, safety_code: String },
    /// Canonical identities, from the group's own MLS state.
    RosterChanged(Vec<String>),
    EpochChanged { epoch: u64, safety_code: String },
    Lost(LostReason),
}
