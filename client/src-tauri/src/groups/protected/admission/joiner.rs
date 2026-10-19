use bytes::Bytes;
use common::errors::AdmissionError;
use common::structs::group_key::AdmissionMessage;
use spake2::{Ed25519Group, Spake2};
use zeroize::Zeroizing;

use super::{AdmissionBinding, GroupPassword, JoinerAwaitingWelcome, PakeContext};

/// The joining side, before the admitter has replied.
pub struct JoinerSession {
    pake: Spake2<Ed25519Group>,
}

impl JoinerSession {
    pub fn start(group_id: &str, password: &GroupPassword) -> (Self, AdmissionMessage) {
        let (pake, outbound) = PakeContext::start(group_id, password);
        (Self { pake }, AdmissionMessage::PakeInit(Bytes::from(outbound)))
    }

    pub fn on_reply(
        self,
        reply: AdmissionMessage,
        key_package: &[u8],
    ) -> Result<(JoinerAwaitingWelcome, AdmissionMessage), AdmissionError> {
        let AdmissionMessage::PakeReply(message) = reply else {
            return Err(AdmissionError::WrongPassword);
        };
        let isk = Zeroizing::new(
            self.pake
                .finish(&message)
                .map_err(|e| AdmissionError::Protocol(e.to_string()))?,
        );
        let offer = AdmissionMessage::KeyPackageOffer {
            key_package: Bytes::copy_from_slice(key_package),
            mac: AdmissionBinding::joiner_mac(&isk, key_package),
        };
        Ok((JoinerAwaitingWelcome::new(isk), offer))
    }
}
