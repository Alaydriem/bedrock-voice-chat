// A host kind was offered after the server started and was refused.
#[derive(Debug, thiserror::Error)]
#[error("host kind is sealed once the server starts")]
pub struct HostKindSealed;
