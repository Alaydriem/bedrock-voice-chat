use bvc_client_lib::websocket::HeadVerdict;

fn upgrade_head() -> String {
    "GET /events?key=abc HTTP/1.1\r\n\
     Host: 127.0.0.1:9595\r\n\
     Connection: Upgrade\r\n\
     Upgrade: websocket\r\n\
     Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n\
     Sec-WebSocket-Version: 13\r\n\r\n"
        .to_string()
}

fn browser_head() -> String {
    "GET /overlay?key=abc HTTP/1.1\r\n\
     Host: 127.0.0.1:9595\r\n\
     User-Agent: Mozilla/5.0\r\n\
     Accept: text/html\r\n\r\n"
        .to_string()
}

/// The existing three routes must be untouched by this. A Stream Deck that upgrades has to
/// reach the handshake exactly as before.
#[test]
fn an_upgrade_request_is_left_to_the_handshake() {
    assert!(matches!(
        HeadVerdict::classify(&upgrade_head()),
        HeadVerdict::Upgrade
    ));
}

/// Header names are case-insensitive, and clients do vary.
#[test]
fn an_upgrade_is_recognised_whatever_its_casing() {
    let head = upgrade_head().replace("Upgrade: websocket", "upgrade: WebSocket");
    assert!(matches!(
        HeadVerdict::classify(&head),
        HeadVerdict::Upgrade
    ));
}

#[test]
fn a_browser_request_is_served_as_http() {
    match HeadVerdict::classify(&browser_head()) {
        HeadVerdict::Http { path, query } => {
            assert_eq!(path, "/overlay");
            assert_eq!(query, "key=abc");
        }
        other => panic!("expected an HTTP verdict, got {other:?}"),
    }
}

#[test]
fn a_request_with_no_query_reports_an_empty_one() {
    let head = browser_head().replace("/overlay?key=abc", "/overlay");
    match HeadVerdict::classify(&head) {
        HeadVerdict::Http { path, query } => {
            assert_eq!(path, "/overlay");
            assert_eq!(query, "");
        }
        other => panic!("expected an HTTP verdict, got {other:?}"),
    }
}

/// The decisive case. A head that stops before the blank line may still name the upgrade in
/// the bytes that have not arrived, so it must not be answered as HTTP.
#[test]
fn a_head_that_has_not_finished_arriving_is_not_yet_decided() {
    let head = upgrade_head();
    let partial = &head[..head.find("Upgrade:").expect("fixture has the header")];
    assert!(matches!(
        HeadVerdict::classify(partial),
        HeadVerdict::Incomplete
    ));
}

/// An upgrade is decidable as soon as the header has arrived, even with the head unfinished.
#[test]
fn an_upgrade_is_decided_as_soon_as_its_header_arrives() {
    let head = upgrade_head();
    let cut = head.find("Sec-WebSocket-Key").expect("fixture has the header");
    assert!(matches!(
        HeadVerdict::classify(&head[..cut]),
        HeadVerdict::Upgrade
    ));
}

#[test]
fn an_empty_read_is_not_yet_decided() {
    assert!(matches!(HeadVerdict::classify(""), HeadVerdict::Incomplete));
}
