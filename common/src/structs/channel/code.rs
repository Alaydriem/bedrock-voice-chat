use rand::RngExt;

/// A group's share code, which is also its id.
///
/// A player reads this off one screen and types it into another, so it is short,
/// case-insensitive, and drawn from an alphabet without the characters that are
/// misread as digits.
pub struct GroupCode;

impl GroupCode {
    /// Crockford base32: no I, L, O or U. The excluded letters are the ones a person
    /// reading a code off a screen substitutes for digits, and U is excluded so a
    /// generated code cannot spell an unwelcome word. Matches `PairingCode`.
    const ALPHABET: &'static [u8] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

    /// Characters per dash-separated group.
    const GROUP: usize = 4;

    /// Total characters, excluding the dash.
    const LEN: usize = 8;

    /// A fresh code, as `XXXX-XXXX`.
    pub fn generate() -> String {
        let mut rng = rand::rng();
        let drawn: String = (0..Self::LEN)
            .map(|_| Self::ALPHABET[rng.random_range(0..Self::ALPHABET.len())] as char)
            .collect();
        format!("{}-{}", &drawn[..Self::GROUP], &drawn[Self::GROUP..])
    }

    /// The canonical form of a code a player typed, or `None` when it is not one.
    ///
    /// Case, spacing and the dash are all discarded before validation, and the
    /// characters the alphabet omits are mapped to the digits they are mistaken for,
    /// so a code read aloud or copied by hand still resolves.
    pub fn normalize(input: &str) -> Option<String> {
        let mut body = String::with_capacity(Self::LEN);
        for c in input.chars() {
            if c.is_whitespace() || c == '-' {
                continue;
            }
            let c = match c.to_ascii_uppercase() {
                'I' | 'L' => '1',
                'O' => '0',
                other => other,
            };
            if !Self::ALPHABET.contains(&(c as u8)) {
                return None;
            }
            body.push(c);
            if body.len() > Self::LEN {
                return None;
            }
        }
        if body.len() != Self::LEN {
            return None;
        }
        Some(format!("{}-{}", &body[..Self::GROUP], &body[Self::GROUP..]))
    }
}
