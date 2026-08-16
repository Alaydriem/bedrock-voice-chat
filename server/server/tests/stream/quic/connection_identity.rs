use bvc_server_lib::stream::quic::{ConnectionClassifier, ConnectionKind};
use common::Game;

#[test]
fn minecraft_player_cn_yields_game_and_bare_name() {
    assert_eq!(
        ConnectionClassifier::classify("minecraft:Steve"),
        ConnectionKind::Player {
            game: Game::Minecraft,
            name: "Steve".to_string(),
        }
    );
}

#[test]
fn hytale_player_cn_yields_game_and_bare_name() {
    assert_eq!(
        ConnectionClassifier::classify("hytale:Alex"),
        ConnectionKind::Player {
            game: Game::Hytale,
            name: "Alex".to_string(),
        }
    );
}

// A `server::`-marked CN must never be admitted as a player.
//
// This shape once had a classification of its own, back when peer links existed.
// The only property that ever mattered is the one asserted here, so it is stated
// as an outcome rather than as a variant.
#[test]
fn a_server_shaped_common_name_is_never_a_player() {
    assert!(!matches!(
        ConnectionClassifier::classify("server::relay.bvc.io:5000"),
        ConnectionKind::Player { .. }
    ));
}

// A `server::`-marked CN that is not a well-formed host:port must never fall
// through to the player path.
#[test]
fn malformed_peer_cn_is_rejected() {
    assert_eq!(
        ConnectionClassifier::classify("server::host:notaport"),
        ConnectionKind::Rejected {
            identity: "server::host:notaport".to_string(),
        }
    );
}

#[test]
fn peer_cn_with_port_zero_is_rejected() {
    assert_eq!(
        ConnectionClassifier::classify("server::host:0"),
        ConnectionKind::Rejected {
            identity: "server::host:0".to_string(),
        }
    );
}

// An unknown game prefix cannot come from a cert this CA issued, so it is
// refused rather than guessed at.
#[test]
fn unknown_game_prefix_is_rejected() {
    assert_eq!(
        ConnectionClassifier::classify("valheim:Bjorn"),
        ConnectionKind::Rejected {
            identity: "valheim:Bjorn".to_string(),
        }
    );
}

// No prefix at all is not a valid issued CN either.
#[test]
fn bare_name_without_game_prefix_is_rejected() {
    assert_eq!(
        ConnectionClassifier::classify("Steve"),
        ConnectionKind::Rejected {
            identity: "Steve".to_string(),
        }
    );
}

// A bare `host:port` without the peer marker must not be mistaken for a peer, and
// `host` is not a game tag, so it is refused.
#[test]
fn bare_host_port_without_marker_is_rejected() {
    assert_eq!(
        ConnectionClassifier::classify("relay.bvc.io:5000"),
        ConnectionKind::Rejected {
            identity: "relay.bvc.io:5000".to_string(),
        }
    );
}

// A player name containing a colon keeps everything after the game tag, so
// names are never truncated.
#[test]
fn player_name_containing_colon_is_preserved() {
    assert_eq!(
        ConnectionClassifier::classify("minecraft:Steve:extra"),
        ConnectionKind::Player {
            game: Game::Minecraft,
            name: "Steve:extra".to_string(),
        }
    );
}

// An empty gamertag is not a usable identity.
#[test]
fn empty_player_name_is_rejected() {
    assert_eq!(
        ConnectionClassifier::classify("minecraft:"),
        ConnectionKind::Rejected {
            identity: "minecraft:".to_string(),
        }
    );
}
