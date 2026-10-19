use common::errors::AdmissionError;
use common::structs::group_key::AdmissionMessage;
use zeroize::Zeroizing;

use super::AdmissionBinding;

/// The joining side once its KeyPackage is offered, waiting for a Welcome it can verify.
pub struct JoinerAwaitingWelcome {
    isk: Zeroizing<Vec<u8>>,
}

impl JoinerAwaitingWelcome {
    pub(crate) fn new(isk: Zeroizing<Vec<u8>>) -> Self {
        Self { isk }
    }

    /// Returns the Welcome only if the admitter proved it holds the same PAKE key. Anything else,
    /// a refusal included, reads as a wrong password: the joiner is never told which side failed.
    pub fn on_welcome(self, offer: AdmissionMessage) -> Result<Vec<u8>, AdmissionError> {
        let AdmissionMessage::WelcomeOffer { welcome, mac } = offer else {
            return Err(AdmissionError::WrongPassword);
        };
        if !AdmissionBinding::verify_admitter(&self.isk, &welcome, &mac) {
            return Err(AdmissionError::WrongPassword);
        }
        Ok(welcome.to_vec())
    }
}
