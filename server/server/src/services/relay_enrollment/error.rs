use common::structs::relay::enroll::EnrollRefuseReason;

#[derive(Debug, thiserror::Error)]
pub enum EnrollmentError {
    #[error("connecting to the relay registry: {0}")]
    Connect(String),
    #[error("relay transport: {0}")]
    Transport(String),
    #[error("the relay refused this server: {explanation}")]
    Refused {
        reason: EnrollRefuseReason,
        explanation: String,
    },
    #[error("the relay speaks no enrollment version this build supports")]
    NoCommonVersion,
    #[error(
        "the relay speaks enrollment version {version}, which has no play names; \
         it needs version 3 or later"
    )]
    PlayNotOffered { version: u16 },
    #[error("expected {expected} from the relay")]
    Unexpected { expected: &'static str },
    #[error(transparent)]
    Wire(#[from] common::errors::PeerWireError),
}

impl EnrollmentError {
    // The sentence an operator reads. The wire carries a coarse reason code, and a
    // bare variant name in a log tells them nothing about what to do next.
    pub fn refused(reason: EnrollRefuseReason) -> Self {
        let explanation = match reason {
            EnrollRefuseReason::NoCommonVersion => {
                "this server and the relay share no enrollment protocol version"
            }
            EnrollRefuseReason::UnknownToken => {
                "the enrollment token is not one the relay issued; get a fresh one"
            }
            EnrollRefuseReason::TokenAlreadyRedeemed => {
                "the enrollment token has already been used; each one enrolls one server"
            }
            EnrollRefuseReason::NotEntitled => {
                "the Discord account that requested this token does not hold a qualifying \
                 membership"
            }
            EnrollRefuseReason::AlreadyRegistered => {
                "that Discord account already holds an assigned name; one is issued per member"
            }
            EnrollRefuseReason::NotRegistered => {
                "this server holds no registration with the relay"
            }
            EnrollRefuseReason::Suspended => {
                "this server's registration is suspended; check the relay for why"
            }
            EnrollRefuseReason::NameNotOwned => {
                "this server asked to act for a name it does not hold"
            }
            EnrollRefuseReason::Internal => {
                "the relay could not complete the request; try again later"
            }
            EnrollRefuseReason::RateLimited => {
                "the relay is limiting new play names from this address; try again later"
            }
            EnrollRefuseReason::NoPlayName => "this server holds no play name with the relay",
            EnrollRefuseReason::CsrRejected => {
                "the relay refused the certificate request for this server's play name"
            }
            EnrollRefuseReason::PlayUnavailable => "the relay does not offer play names",
        };

        Self::Refused {
            reason,
            explanation: explanation.to_string(),
        }
    }

    // The registry's reason, when this is a refusal. Callers act on the reason rather than
    // on the sentence.
    pub fn reason(&self) -> Option<EnrollRefuseReason> {
        match self {
            Self::Refused { reason, .. } => Some(*reason),
            _ => None,
        }
    }
}
