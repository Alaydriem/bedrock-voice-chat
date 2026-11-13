use serde::{Deserialize, Serialize};

// The registry's verdict on a play-mode server's public address.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlayPreflightOutcome {
    Reachable { address: String, port: u16 },
    AddressNotPublic { address: String },
    AddressCgnat { address: String },
    TimedOut { address: String, port: u16 },
    Refused { address: String, port: u16 },
    WrongService { address: String, port: u16 },
    RateLimited { retry_after_secs: u64 },
}

impl PlayPreflightOutcome {
    // The variant's name, for log fields that are filtered by outcome.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Reachable { .. } => "Reachable",
            Self::AddressNotPublic { .. } => "AddressNotPublic",
            Self::AddressCgnat { .. } => "AddressCgnat",
            Self::TimedOut { .. } => "TimedOut",
            Self::Refused { .. } => "Refused",
            Self::WrongService { .. } => "WrongService",
            Self::RateLimited { .. } => "RateLimited",
        }
    }
}
