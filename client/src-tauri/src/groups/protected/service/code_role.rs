/// Which side of a check a code is shown on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodeRole {
    /// The person asking to join, who reads the code out.
    Joiner,
    /// The member checking them, who admits on a match.
    Member,
}

impl CodeRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Joiner => "joiner",
            Self::Member => "member",
        }
    }
}
