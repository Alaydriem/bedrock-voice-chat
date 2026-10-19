use hkdf::Hkdf;
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};
use zeroize::Zeroizing;

type HmacSha256 = Hmac<Sha256>;

/// Binds the PAKE's shared key to the exact MLS object each side vouches for.
///
/// Without these, a relay could let a real joiner pass the PAKE and then swap in its own
/// KeyPackage, or hand the joiner a Welcome into a group the relay runs. Each MAC covers the hash
/// of one object under a key derived for one direction, so neither can be moved to the other.
pub struct AdmissionBinding;

impl AdmissionBinding {
    const JOINER: &'static [u8] = b"bvc-admission-v1 joiner";
    const ADMITTER: &'static [u8] = b"bvc-admission-v1 admitter";

    pub fn joiner_mac(isk: &[u8], key_package: &[u8]) -> [u8; 32] {
        Self::mac(isk, Self::JOINER, key_package)
    }

    pub fn admitter_mac(isk: &[u8], welcome: &[u8]) -> [u8; 32] {
        Self::mac(isk, Self::ADMITTER, welcome)
    }

    pub fn verify_joiner(isk: &[u8], key_package: &[u8], mac: &[u8; 32]) -> bool {
        Self::verify(isk, Self::JOINER, key_package, mac)
    }

    pub fn verify_admitter(isk: &[u8], welcome: &[u8], mac: &[u8; 32]) -> bool {
        Self::verify(isk, Self::ADMITTER, welcome, mac)
    }

    fn keyed(isk: &[u8], label: &[u8], object: &[u8]) -> HmacSha256 {
        let mut key = Zeroizing::new([0u8; 32]);
        Hkdf::<Sha256>::new(None, isk)
            .expand(label, key.as_mut())
            .expect("32 bytes is a valid HKDF-SHA256 output length");
        let mut mac =
            HmacSha256::new_from_slice(key.as_ref()).expect("HMAC accepts any key length");
        mac.update(&Sha256::digest(object));
        mac
    }

    fn mac(isk: &[u8], label: &[u8], object: &[u8]) -> [u8; 32] {
        Self::keyed(isk, label, object).finalize().into_bytes().into()
    }

    fn verify(isk: &[u8], label: &[u8], object: &[u8], mac: &[u8; 32]) -> bool {
        Self::keyed(isk, label, object).verify_slice(mac).is_ok()
    }
}
