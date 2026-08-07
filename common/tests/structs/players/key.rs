use common::structs::audio::PlayerGainSettings;
use common::structs::players::PlayerKey;

#[test]
fn round_trips_through_its_encoding() {
    let key = PlayerKey::new("bvc.alaydriem.com", "minecraft:Alaydriem");
    let decoded = PlayerKey::decode(&key.encode()).expect("decodes");
    assert_eq!(decoded, key);
}

// The encoding is what redb stores, so changing it is a silent data migration: every existing
// row becomes unfindable and every setting reads as unity. A round-trip cannot catch that —
// swap the separator and it still passes — so the literal is pinned here on purpose. If this
// test fails, the fix is a migration, not a new expected value.
#[test]
fn encodes_to_exactly_the_bytes_already_on_disk() {
    assert_eq!(
        PlayerKey::new("bvc.alaydriem.com", "minecraft:Alaydriem").encode(),
        "bvc.alaydriem.com\u{1f}minecraft:Alaydriem"
    );
    assert_eq!(PlayerKey::server_prefix("bvc.alaydriem.com"), "bvc.alaydriem.com\u{1f}");
}

// A gamertag can contain spaces and a host can contain dots and colons. Neither can
// contain the unit separator, which is why it is the separator.
#[test]
fn survives_a_name_with_spaces_and_a_host_with_a_port() {
    let key = PlayerKey::new("voice.example.com:8443", "minecraft:Some Gamer");
    assert_eq!(PlayerKey::decode(&key.encode()), Some(key));
}

// Listing one server's players is a prefix range scan in redb, so every key for a server
// has to share a prefix that no other server's keys can start with.
#[test]
fn keys_for_one_server_share_a_scannable_prefix() {
    let prefix = PlayerKey::server_prefix("bvc.alaydriem.com");
    assert!(
        PlayerKey::new("bvc.alaydriem.com", "minecraft:Al")
            .encode()
            .starts_with(&prefix)
    );
    assert!(
        !PlayerKey::new("bvc.alaydriem.com.evil.test", "minecraft:Al")
            .encode()
            .starts_with(&prefix)
    );
}

#[test]
fn rejects_a_malformed_encoding() {
    assert_eq!(PlayerKey::decode("no-separator-here"), None);
}

// Unity gain and unmuted is what proximity writes for everybody, so it is the state that
// means "no decision has been made" — and the Players pane leads with the ones that have.
// A proximity stamp alone is not a decision.
#[test]
fn knows_which_settings_carry_a_decision() {
    assert!(!PlayerGainSettings::unity().is_adjusted());
    assert!(
        !PlayerGainSettings {
            gain: 1.0,
            muted: false,
            last_seen: Some(1.0)
        }
        .is_adjusted()
    );
    assert!(
        PlayerGainSettings {
            gain: 1.0,
            muted: true,
            last_seen: None
        }
        .is_adjusted()
    );
    assert!(
        PlayerGainSettings {
            gain: 0.5,
            muted: false,
            last_seen: None
        }
        .is_adjusted()
    );
}
