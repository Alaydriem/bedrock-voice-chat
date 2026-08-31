use common::structs::relay::enroll::EnrollRefuseReason;

#[derive(Debug, thiserror::Error)]
pub enum EnrollmentError {
    #[error("connecting to the relay registry: {0}")]
    Connect(String),
    #[error("relay transport: {0}")]
    Transport(String),
    #[error("the relay refused this server: {0}")]
    Refused(String),
    #[error("the relay speaks no enrollment version this build supports")]
    NoCommonVersion,
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
        };

        Self::Refused(explanation.to_string())
    }
}
