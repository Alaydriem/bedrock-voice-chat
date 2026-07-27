use super::ConnectionKind;
use common::Game;

pub struct ConnectionClassifier;

impl ConnectionClassifier {
    // Classifies an mTLS-authenticated certificate Common Name.
    //
    // `server::{host}:{port}` is a peer when the remainder is a well-formed
    // `host:port` (non-empty host, non-zero u16 port). `{game}:{playername}` is a
    // player when the prefix is a known game tag and the name is non-empty.
    // Everything else is refused, so an unrecognized CN never reaches the player
    // path by default.
    pub fn classify(identity: &str) -> ConnectionKind {
        if let Some(rest) = identity.strip_prefix("server::") {
            return Self::classify_peer(identity, rest);
        }

        match identity.split_once(':') {
            Some((tag, name)) => match (Game::from_tag(tag), name.is_empty()) {
                (Some(game), false) => ConnectionKind::Player {
                    game,
                    name: name.to_string(),
                },
                _ => Self::rejected(identity),
            },
            None => Self::rejected(identity),
        }
    }

    // Splits on the LAST colon so namespaced/IPv6-ish hosts survive.
    fn classify_peer(identity: &str, rest: &str) -> ConnectionKind {
        if let Some((host, suffix)) = rest.rsplit_once(':') {
            if !host.is_empty() {
                if let Ok(port) = suffix.parse::<u16>() {
                    if port != 0 {
                        return ConnectionKind::Peer {
                            host: host.to_string(),
                            port,
                            endpoint: format!("{host}:{port}"),
                        };
                    }
                }
            }
        }

        Self::rejected(identity)
    }

    fn rejected(identity: &str) -> ConnectionKind {
        ConnectionKind::Rejected {
            identity: identity.to_string(),
        }
    }
}
