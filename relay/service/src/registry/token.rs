use base64::Engine;

// The value an operator pastes into their config exactly once.
//
// Prefixed so a value pasted into the wrong field is refused with a sentence about
// what it is, rather than failing later as an unknown token.
pub struct EnrollmentToken;

impl EnrollmentToken {
    pub const PREFIX: &'static str = "bvcenroll";

    // How long an issued token stays redeemable. Long enough for an operator to
    // finish editing a config file, short enough that one left in a chat log is not
    // a standing credential.
    pub const TTL_SECONDS: i64 = 24 * 60 * 60;

    pub fn mint() -> String {
        let mut bytes = [0u8; 32];
        getrandom::fill(&mut bytes).expect("the system random source is available");
        format!(
            "{}{}",
            Self::PREFIX,
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
        )
    }
}
