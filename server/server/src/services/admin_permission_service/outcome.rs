/// The discriminants are the `bvc_admin` return codes the embedding mod decodes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum AdminPermissionOutcome {
    Applied = 0,
    NothingToRevoke = 1,
    PlayerNotFound = 2,
}
