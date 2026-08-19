use bvc_client_lib::CredentialWriteSet;
use common::response::LoginResponse;
use common::structs::config::Keypair;

fn keypair() -> Keypair {
    Keypair {
        pk: vec![1, 2, 3],
        sk: vec![4, 5, 6],
    }
}

fn response() -> LoginResponse {
    LoginResponse {
        gamerpic: "https://example.invalid/pic.png".to_string(),
        gamertag: "Alaydriem".to_string(),
        keypair: keypair(),
        signature: keypair(),
        certificate: "cert-pem".to_string(),
        certificate_key: "cert-key-pem".to_string(),
        certificate_ca: "ca-pem".to_string(),
        quic_connect_string: "quic://example.invalid:443".to_string(),
        minecraft_username: None,
        server_permissions: None,
        game: None,
    }
}

// The whole point of the set: every value is produced before the first write, so a serialization
// failure cannot leave a partial identity in the keystore. If the eight mandatory fields are not
// all present here, some of them are still being serialized inside the write loop.
#[test]
fn the_write_set_carries_every_mandatory_field() {
    let set = CredentialWriteSet::build(&response()).expect("build should succeed");
    let keys: Vec<&str> = set.iter().map(|(key, _)| *key).collect();

    for key in [
        "gamerpic",
        "gamertag",
        "keypair",
        "signature",
        "certificate",
        "certificate_key",
        "certificate_ca",
        "quic_connect_string",
    ] {
        assert!(keys.contains(&key), "{key} should be in the write set");
    }
}

// A response with no permissions, no Minecraft username and no game must not produce empty entries
// for them. An empty string read back later is indistinguishable from a real value, where an
// absent key is a clean miss.
#[test]
fn absent_optional_fields_produce_no_entries() {
    let set = CredentialWriteSet::build(&response()).expect("build should succeed");
    let keys: Vec<&str> = set.iter().map(|(key, _)| *key).collect();

    assert_eq!(keys.len(), 8);
    assert!(!keys.contains(&"server_permissions"));
    assert!(!keys.contains(&"minecraft_username"));
    assert!(!keys.contains(&"game"));
}

// `game` is the one part of an identity a code login cannot reconstruct, so a present game must
// reach the write set rather than being left to the caller.
#[test]
fn a_present_game_is_included() {
    let mut with_game = response();
    with_game.game = Some(common::Game::Minecraft);

    let set = CredentialWriteSet::build(&with_game).expect("build should succeed");
    let keys: Vec<&str> = set.iter().map(|(key, _)| *key).collect();

    assert!(keys.contains(&"game"));
}

// gamerpic is written first today and the first write is the one that failed on Ubuntu, so the
// order is load-bearing for reproducing the report. Pinned so a reorder is deliberate.
#[test]
fn gamerpic_is_written_first() {
    let set = CredentialWriteSet::build(&response()).expect("build should succeed");

    assert_eq!(set[0].0, "gamerpic");
}
