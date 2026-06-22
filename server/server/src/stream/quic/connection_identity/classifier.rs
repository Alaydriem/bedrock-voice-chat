use super::ConnectionKind;

pub struct ConnectionClassifier;

impl ConnectionClassifier {
    // Classifies an authenticated connection identity string.
    //
    // A server-peer identity carries the explicit marker `server::{host}:{port}`
    // (issued by `sign_peer_cert`). When the marker is present and the remainder
    // is a well-formed `host:port` (non-empty host, non-zero u16 port), the
    // connection is a `Peer`. When the marker is present but the remainder is
    // malformed, the connection is `Rejected` (fail closed) — it is NEVER treated
    // as a player. Any identity WITHOUT the marker (including the player shape
    // `{game}:{playername}`) is a `Player` and takes the normal client path.
    //
    // Because the marker is explicit, a peer CN can never be mistaken for a player
    // and a bare `host:port` (no marker) is a player, not a peer.
    pub fn classify(identity: &str) -> ConnectionKind {
        let Some(rest) = identity.strip_prefix("server::") else {
            return ConnectionKind::Player {
                identity: identity.to_string(),
            };
        };

        // `server::` marker present: the remainder must be a well-formed
        // `host:port`. Split on the LAST colon so namespaced/IPv6-ish hosts work.
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

        // Marked as a peer but not a well-formed endpoint: refuse, never a player.
        ConnectionKind::Rejected {
            identity: identity.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn server_peer_cn_parses_host_and_port() {
        assert_eq!(
            ConnectionClassifier::classify("server::relay.bvc.io:5000"),
            ConnectionKind::Peer {
                host: "relay.bvc.io".to_string(),
                port: 5000,
                endpoint: "relay.bvc.io:5000".to_string(),
            }
        );
    }

    #[test]
    fn server_peer_cn_with_ipv4_host() {
        assert_eq!(
            ConnectionClassifier::classify("server::203.0.113.7:5001"),
            ConnectionKind::Peer {
                host: "203.0.113.7".to_string(),
                port: 5001,
                endpoint: "203.0.113.7:5001".to_string(),
            }
        );
    }

    #[test]
    fn server_peer_cn_multicolon_host_uses_last_segment_as_port() {
        assert_eq!(
            ConnectionClassifier::classify("server::a:b:5000"),
            ConnectionKind::Peer {
                host: "a:b".to_string(),
                port: 5000,
                endpoint: "a:b:5000".to_string(),
            }
        );
    }

    #[test]
    fn minecraft_player_cn_is_player() {
        assert_eq!(
            ConnectionClassifier::classify("minecraft:Steve"),
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
    fn player_name_with_colon_is_player() {
        assert_eq!(
            ConnectionClassifier::classify("minecraft:Steve:5000"),
            ConnectionKind::Player {
                identity: "minecraft:Steve:5000".to_string()
            }
        );
    }

    // Closes the masquerade gap: a bare `host:port` WITHOUT the `server::` marker
    // is a player (normal client path), never a peer.
    #[test]
    fn bare_host_port_without_marker_is_player() {
        assert_eq!(
            ConnectionClassifier::classify("relay.bvc.io:5000"),
            ConnectionKind::Player {
                identity: "relay.bvc.io:5000".to_string()
            }
        );
    }

    #[test]
    fn no_colon_is_player() {
        assert_eq!(
            ConnectionClassifier::classify("nocolon"),
            ConnectionKind::Player {
                identity: "nocolon".to_string()
            }
        );
    }

    // A `server::`-marked CN that is not a well-formed host:port must fail closed
    // (Rejected), NOT default to Player.
    #[test]
    fn server_marker_non_numeric_port_is_rejected() {
        assert_eq!(
            ConnectionClassifier::classify("server::host:notaport"),
            ConnectionKind::Rejected {
                identity: "server::host:notaport".to_string()
            }
        );
    }

    #[test]
    fn server_marker_port_zero_is_rejected() {
        assert_eq!(
            ConnectionClassifier::classify("server::host:0"),
            ConnectionKind::Rejected {
                identity: "server::host:0".to_string()
            }
        );
    }

    #[test]
    fn server_marker_empty_host_is_rejected() {
        assert_eq!(
            ConnectionClassifier::classify("server:::5000"),
            ConnectionKind::Rejected {
                identity: "server:::5000".to_string()
            }
        );
    }

    #[test]
    fn server_marker_no_port_is_rejected() {
        assert_eq!(
            ConnectionClassifier::classify("server::nocolon"),
            ConnectionKind::Rejected {
                identity: "server::nocolon".to_string()
            }
        );
    }
}
