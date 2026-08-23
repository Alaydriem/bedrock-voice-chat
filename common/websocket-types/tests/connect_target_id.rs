use websocket_types::{ConnectTargetId, ConnectTargetSource};

#[test]
fn mints_one_id_per_source() {
    assert_eq!(
        ConnectTargetId::mint(ConnectTargetSource::Saved, "V1StGXR8_Z5jdHi6B"),
        "saved:V1StGXR8_Z5jdHi6B"
    );
    assert_eq!(
        ConnectTargetId::mint(ConnectTargetSource::Realm, "1234567"),
        "realm:1234567"
    );
}

// A server entry's native half is `host:port`, so it carries a colon of its own. Splitting
// on the last colon, or on every colon, would hand the connect path a truncated hostname.
#[test]
fn parses_a_server_id_without_splitting_its_host_and_port() {
    let minted = ConnectTargetId::mint(ConnectTargetSource::Server, "play.example.com:19132");
    let (source, native) = ConnectTargetId::parse(&minted).expect("server id should parse");

    assert_eq!(source, ConnectTargetSource::Server);
    assert_eq!(native, "play.example.com:19132");
}

// A bare id cannot be routed: a saved proxy's nanoid and a realm's numeric id are both
// opaque strings. An id that arrives without a source it recognises is refused rather than
// guessed at, or a controller quoting a stale id connects to the wrong world.
#[test]
fn refuses_an_unknown_source() {
    assert!(ConnectTargetId::parse("channel:1234").is_none());
}

#[test]
fn refuses_an_id_with_no_source() {
    assert!(ConnectTargetId::parse("1234567").is_none());
}

#[test]
fn refuses_an_id_with_an_empty_native_half() {
    assert!(ConnectTargetId::parse("realm:").is_none());
}
