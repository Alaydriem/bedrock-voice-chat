use serde::{Deserialize, Serialize};

/// Why a group could not be turned into a protected group.
///
/// `as_code` is the body the protect route returns, so the app can tell the two conflicts apart.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
pub enum ProtectRefusal {
    #[error("the group does not exist")]
    NotFound,
    #[error("only the group's creator can protect it")]
    NotCreator,
    #[error("the group is already protected")]
    AlreadyProtected,
    #[error("someone else is in the group")]
    NotAlone,
}

impl ProtectRefusal {
    pub fn as_code(&self) -> &'static str {
        match self {
            Self::NotFound => "not_found",
            Self::NotCreator => "not_creator",
            Self::AlreadyProtected => "already_protected",
            Self::NotAlone => "not_alone",
        }
    }
}
