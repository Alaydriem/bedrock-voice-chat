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
