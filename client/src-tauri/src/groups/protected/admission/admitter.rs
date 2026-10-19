use bytes::Bytes;
use common::errors::AdmissionError;
use common::structs::group_key::AdmissionMessage;
use zeroize::Zeroizing;

use super::{AdmissionBinding, GroupPassword, PakeContext};

/// A member's client admitting one joiner. One session per joiner, so concurrent joins never
/// share state.
pub struct AdmitterSession {
    isk: Zeroizing<Vec<u8>>,
}

impl AdmitterSession {
    pub fn respond(
        group_id: &str,
        password: &GroupPassword,
        init: AdmissionMessage,
    ) -> Result<(Self, AdmissionMessage), AdmissionError> {
        let AdmissionMessage::PakeInit(message) = init else {
            return Err(AdmissionError::Protocol("expected a PAKE init".to_string()));
        };
        let (pake, outbound) = PakeContext::start(group_id, password);
        let isk = Zeroizing::new(
            pake.finish(&message)
                .map_err(|e| AdmissionError::Protocol(e.to_string()))?,
        );
        Ok((Self { isk }, AdmissionMessage::PakeReply(Bytes::from(outbound))))
    }

    /// Returns the KeyPackage only if its MAC proves it came from someone who knows the password.
    pub fn verify_offer(&self, offer: AdmissionMessage) -> Result<Vec<u8>, AdmissionError> {
        let AdmissionMessage::KeyPackageOffer { key_package, mac } = offer else {
            return Err(AdmissionError::Protocol("expected a key package offer".to_string()));
        };
        if !AdmissionBinding::verify_joiner(&self.isk, &key_package, &mac) {
            return Err(AdmissionError::WrongPassword);
        }
        Ok(key_package.to_vec())
    }

    pub fn seal_welcome(self, welcome: &[u8]) -> AdmissionMessage {
        AdmissionMessage::WelcomeOffer {
            welcome: Bytes::copy_from_slice(welcome),
            mac: AdmissionBinding::admitter_mac(&self.isk, welcome),
        }
    }
}
