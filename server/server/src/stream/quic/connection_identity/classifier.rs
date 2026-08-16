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
        // `server::` was a peer marker while peer links existed. It is not a game
        // tag, so it now falls through to the same refusal as any other unknown
        // prefix — but it is named here so that stays deliberate rather than
        // incidental.
        if identity.starts_with("server::") {
            return Self::rejected(identity);
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

    fn rejected(identity: &str) -> ConnectionKind {
        ConnectionKind::Rejected {
            identity: identity.to_string(),
        }
    }
}
