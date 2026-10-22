use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
pub enum ProtectedGroupError {
    #[error("a protected group can only be joined from the BVC app")]
    RequiresClient,
    #[error("this client does not support protected groups")]
    UnsupportedClient,
    #[error("no member is online to admit you")]
    NoAdmitterOnline,
    #[error("the group is not protected")]
    NotProtected,
    #[error("not a member of the group")]
    NotMember,
    #[error("key socket frame exceeds the size limit")]
    FrameTooLarge,
    #[error("key socket exceeded its rate limit")]
    RateLimited,
}
