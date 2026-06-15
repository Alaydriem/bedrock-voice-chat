use crate::identity::IdentitySlot;

use super::IdentityMetadata;

#[derive(Debug, Clone)]
pub struct IdentitySummary {
    pub slot: IdentitySlot,
    #[warn(dead_code)]
    pub metadata: IdentityMetadata,
}
