use super::ConnectionKind;
use common::Game;

pub struct ConnectionClassifier;

impl ConnectionClassifier {
    // Classifies an authenticated connection identity string.
    //
    // The game prefix is ALWAYS the first segment of a player identity
    // (`{game}:{playername}`, issued by `sign_player_cert`). So the head — the
    // text before the FIRST `:` — is the strongest available signal: if it is a
    // known game keyword the connection is ALWAYS a `Player`, no matter what the
    // remainder contains (a display name may itself contain colons, e.g.
    // `minecraft:Steve:5000`). Only when the head is NOT a known game AND the
    // identity's LAST segment parses as a non-zero u16 port is it treated as a
    // `Peer`. Any ambiguity falls through to the safe `Player` default, so a
    // mis-classification can never silently swallow a real player's connection.
    pub fn classify(identity: &str) -> ConnectionKind {
        if let Some((head, _)) = identity.split_once(':') {
            // First segment is a known game keyword -> always a player, never a
            // peer, regardless of the rest of the identity.
            if Self::is_known_game(head) {
                return ConnectionKind::Player {
                    identity: identity.to_string(),
                };
            }
        }

        // Not game-prefixed: a peer iff the last segment is a non-zero u16 port
        // and the host portion (everything before the last colon) is non-empty.
        if let Some((host, suffix)) = identity.rsplit_once(':') {
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

        ConnectionKind::Player {
            identity: identity.to_string(),
        }
    }

    // Whether `prefix` is one of the known game keywords `sign_player_cert`
    // emits. Matched against the `Game` enum's canonical strings so adding a
    // game here is covered without touching this classifier.
    fn is_known_game(prefix: &str) -> bool {
        [Game::Minecraft, Game::Hytale]
            .iter()
            .any(|g| g.as_str() == prefix)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn peer_cn_parses_host_and_port() {
        let kind = ConnectionClassifier::classify("relay.bvc.io:5000");
        assert_eq!(
            kind,
            ConnectionKind::Peer {
                host: "relay.bvc.io".to_string(),
                port: 5000,
                endpoint: "relay.bvc.io:5000".to_string(),
            }
        );
    }

    #[test]
    fn peer_cn_with_ipv4_host() {
        let kind = ConnectionClassifier::classify("203.0.113.7:5001");
        assert_eq!(
            kind,
            ConnectionKind::Peer {
                host: "203.0.113.7".to_string(),
                port: 5001,
                endpoint: "203.0.113.7:5001".to_string(),
            }
        );
    }

    #[test]
    fn minecraft_player_cn_is_player() {
        let kind = ConnectionClassifier::classify("minecraft:Steve");
        assert_eq!(
            kind,
            ConnectionKind::Player {
                identity: "minecraft:Steve".to_string()
            }
        );
    }

    #[test]
    fn hytale_player_cn_is_player() {
        assert_eq!(
            ConnectionClassifier::classify("hytale:Alex"),
            ConnectionKind::Player {
                identity: "hytale:Alex".to_string()
            }
        );
    }

    #[test]
    fn numeric_gamertag_after_known_game_is_player() {
        // a game-prefixed numeric name must not be mistaken for host:port
        assert_eq!(
            ConnectionClassifier::classify("hytale:12345"),
            ConnectionKind::Player {
                identity: "hytale:12345".to_string()
            }
        );
    }

    #[test]
    fn minecraft_player_with_colon_in_name_is_player() {
        // a display name containing a colon (and a numeric tail that looks like
        // a port) must STILL classify as a player because the head is a known
        // game keyword.
        assert_eq!(
            ConnectionClassifier::classify("minecraft:Steve:5000"),
            ConnectionKind::Player {
                identity: "minecraft:Steve:5000".to_string()
            }
        );
    }

    #[test]
    fn hytale_player_with_colon_in_name_is_player() {
        assert_eq!(
            ConnectionClassifier::classify("hytale:Name:1"),
            ConnectionKind::Player {
                identity: "hytale:Name:1".to_string()
            }
        );
    }

    #[test]
    fn malformed_no_colon_defaults_to_player() {
        assert_eq!(
            ConnectionClassifier::classify("nocolon"),
            ConnectionKind::Player {
                identity: "nocolon".to_string()
            }
        );
    }

    #[test]
    fn non_numeric_suffix_defaults_to_player() {
        assert_eq!(
            ConnectionClassifier::classify("host:notaport"),
            ConnectionKind::Player {
                identity: "host:notaport".to_string()
            }
        );
    }

    #[test]
    fn port_zero_defaults_to_player() {
        assert_eq!(
            ConnectionClassifier::classify("host:0"),
            ConnectionKind::Player {
                identity: "host:0".to_string()
            }
        );
    }

    #[test]
    fn empty_host_defaults_to_player() {
        assert_eq!(
            ConnectionClassifier::classify(":5000"),
            ConnectionKind::Player {
                identity: ":5000".to_string()
            }
        );
    }

    #[test]
    fn host_with_multiple_colons_uses_last_segment_as_port() {
        // e.g. an IPv6-ish or namespaced host — split on the LAST colon
        let kind = ConnectionClassifier::classify("a:b:5000");
        assert_eq!(
            kind,
            ConnectionKind::Peer {
                host: "a:b".to_string(),
                port: 5000,
                endpoint: "a:b:5000".to_string(),
            }
        );
    }
}
