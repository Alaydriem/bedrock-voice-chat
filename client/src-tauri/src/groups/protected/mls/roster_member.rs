/// One member of a protected group, as the group's own MLS state records it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RosterMember {
    pub leaf: u32,
    pub identity: common::PlayerIdentity,
}
