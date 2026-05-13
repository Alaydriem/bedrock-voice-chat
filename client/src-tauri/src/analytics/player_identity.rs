use rustc_hash::FxHasher;
use std::hash::Hasher;

#[derive(Debug, Clone)]
pub struct PlayerIdentity {
    pub display: String,
    pub hash: String,
}

impl PlayerIdentity {
    pub fn from_gamertag(gamertag: &str) -> Self {
        Self {
            display: Self::censor(gamertag),
            hash: Self::fx_hash_hex(gamertag),
        }
    }

    fn censor(gamertag: &str) -> String {
        let chars: Vec<char> = gamertag.chars().collect();
        let len = chars.len();

        if len < 6 {
            return "*".repeat(len);
        }

        let prefix: String = chars[..3].iter().collect();
        let suffix: String = chars[len - 2..].iter().collect();
        let middle = "*".repeat(len - 5);
        format!("{prefix}{middle}{suffix}")
    }

    fn fx_hash_hex(gamertag: &str) -> String {
        let mut hasher = FxHasher::default();
        hasher.write(gamertag.to_lowercase().as_bytes());
        format!("{:016x}", hasher.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn censor_long_gamertag() {
        let id = PlayerIdentity::from_gamertag("Charlie123");
        assert_eq!(id.display, "Cha*****23");
    }

    #[test]
    fn censor_seven_char_gamertag() {
        let id = PlayerIdentity::from_gamertag("Charles");
        assert_eq!(id.display, "Cha**es");
    }

    #[test]
    fn censor_six_char_gamertag() {
        let id = PlayerIdentity::from_gamertag("Charle");
        assert_eq!(id.display, "Cha*le");
    }

    #[test]
    fn censor_short_gamertag() {
        let id = PlayerIdentity::from_gamertag("Bob");
        assert_eq!(id.display, "***");
    }

    #[test]
    fn censor_empty_gamertag() {
        let id = PlayerIdentity::from_gamertag("");
        assert_eq!(id.display, "");
    }

    #[test]
    fn hash_is_stable() {
        let a = PlayerIdentity::from_gamertag("Charles");
        let b = PlayerIdentity::from_gamertag("Charles");
        assert_eq!(a.hash, b.hash);
        assert_eq!(a.hash.len(), 16);
    }

    #[test]
    fn hash_is_case_insensitive() {
        let a = PlayerIdentity::from_gamertag("Charles");
        let b = PlayerIdentity::from_gamertag("CHARLES");
        assert_eq!(a.hash, b.hash);
    }

    #[test]
    fn hash_differs_between_gamertags() {
        let a = PlayerIdentity::from_gamertag("Charles");
        let b = PlayerIdentity::from_gamertag("Charlie");
        assert_ne!(a.hash, b.hash);
    }
}
