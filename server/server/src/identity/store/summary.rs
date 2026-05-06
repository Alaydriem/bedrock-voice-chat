use crate::identity::IdentitySlot;

use super::IdentityMetadata;

#[derive(Debug, Clone)]
pub struct IdentitySummary {
    pub slot: IdentitySlot,
    pub metadata: IdentityMetadata,
}
