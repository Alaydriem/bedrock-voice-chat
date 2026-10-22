use openmls::prelude::StagedWelcome;

/// A Welcome read but not yet entered: who signed it and which group it is for can be checked
/// before the joiner takes the group's secrets.
pub struct StagedJoin {
    pub(super) staged: StagedWelcome,
}

impl StagedJoin {
    /// The signature key of the member that built the Welcome. `None` if the Welcome names a
    /// sender that is not in its own tree.
    pub fn sender_signature_key(&self) -> Option<Vec<u8>> {
        self.staged
            .welcome_sender()
            .ok()
            .map(|leaf| leaf.signature_key().as_slice().to_vec())
    }

    pub fn mls_group_id(&self) -> Vec<u8> {
        self.staged.group_context().group_id().as_slice().to_vec()
    }
}
