#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdminVerb {
    Grant,
    Revoke,
}

impl AdminVerb {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Grant => "grant",
            Self::Revoke => "revoke",
        }
    }
}
