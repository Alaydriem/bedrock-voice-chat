// Where a registration sits in its lifecycle.
//
// Stored as a string rather than an integer: the column is read by operators during
// incidents, and a number would need a lookup table nobody has to hand.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegistrationState {
    Pending,
    Active,
    Suspended,
    Retired,
}

impl RegistrationState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Active => "active",
            Self::Suspended => "suspended",
            Self::Retired => "retired",
        }
    }

    pub fn from_str(value: &str) -> Option<Self> {
        match value {
            "pending" => Some(Self::Pending),
            "active" => Some(Self::Active),
            "suspended" => Some(Self::Suspended),
            "retired" => Some(Self::Retired),
            _ => None,
        }
    }
}
