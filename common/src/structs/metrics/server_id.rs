// The join key between a client's link report and a server's own fleet events.
//
// Both sides derive it from the CA certificate, which every deployment must already have and
// which survives restarts. The hazard this type exists to remove: the server reads the CA from
// disk while a client holds it as a string from its credential store, and a single trailing
// newline between those two produces a different hash — so the join silently never closes, with
// no error anywhere to indicate why. Trailing whitespace is therefore stripped before hashing,
// in one place, so the two sides cannot drift apart.
pub struct ServerId;

impl ServerId {
    pub fn from_ca_pem(pem: &[u8]) -> String {
        let trimmed = Self::trim_trailing_whitespace(pem);
        blake3::hash(trimmed).to_hex().to_string()
    }

    fn trim_trailing_whitespace(bytes: &[u8]) -> &[u8] {
        let mut end = bytes.len();
        while end > 0 && bytes[end - 1].is_ascii_whitespace() {
            end -= 1;
        }
        &bytes[..end]
    }
}
