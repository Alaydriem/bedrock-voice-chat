use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// The unit separator. Chosen because neither half can contain it: a hostname is drawn from
/// the LDH set plus a port, and a gamertag from a game's own character set. Both exclude C0
/// controls, so the split is unambiguous without escaping either side.
const SEPARATOR: char = '\u{1f}';

/// Identifies one player's settings on one server.
///
/// `cn` is the canonical `game:gamertag` — `minecraft:Alaydriem` — which is the certificate
/// CN the server stamps onto every packet. It is never a bare gamertag. A bare name does not
/// resolve, and an unresolved lookup reads as unity gain rather than as an error, so a keying
/// mistake is audible instead of a silent absence nobody can debug.
///
/// The server is part of the key because a decision is about a person *here*. The same
/// gamertag on two servers is two rows, and muting them on one must not silence them on the
/// other.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, TS,
)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct PlayerKey {
    pub server: String,
    pub cn: String,
}

impl PlayerKey {
    pub fn new(server: impl Into<String>, cn: impl Into<String>) -> Self {
        Self {
            server: server.into(),
            cn: cn.into(),
        }
    }

    /// The redb key. Changing this format is a silent data migration: old rows stop being
    /// found and every setting reads as unity.
    pub fn encode(&self) -> String {
        format!("{}{}{}", self.server, SEPARATOR, self.cn)
    }

    pub fn decode(encoded: &str) -> Option<Self> {
        encoded
            .split_once(SEPARATOR)
            .map(|(server, cn)| Self::new(server, cn))
    }

    /// The prefix every key for one server shares, and no other server's keys can start with.
    ///
    /// The trailing separator is what makes that true: without it `bvc.example.com` would also
    /// prefix `bvc.example.com.evil.test`, and a range scan for one server would read another
    /// server's rows.
    pub fn server_prefix(server: &str) -> String {
        format!("{server}{SEPARATOR}")
    }
}
