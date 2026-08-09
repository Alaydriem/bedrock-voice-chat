use websocket_types::{ConnectTarget, ConnectTargetKind};

fn proxy(id: &str, name: &str, host: Option<&str>, port: Option<u16>) -> ConnectTarget {
    ConnectTarget {
        id: id.to_string(),
        name: name.to_string(),
        kind: ConnectTargetKind::Proxy,
        host: host.map(String::from),
        port,
        protocol_version: None,
    }
}

fn realm(id: &str, name: &str) -> ConnectTarget {
    ConnectTarget {
        id: id.to_string(),
        name: name.to_string(),
        kind: ConnectTargetKind::Realm,
        host: None,
        port: None,
        protocol_version: None,
    }
}

// A list and a connect are two calls, so the list is a snapshot the caller acts on later.
// Matching is by id and never by list position, or an entry added between the two calls
// connects the operator to the wrong world.
#[test]
fn find_matches_on_id_not_position() {
    let targets = vec![
        proxy("proxy-a", "Local BDS", Some("127.0.0.1"), Some(19132)),
        realm("1234", "My Realm"),
    ];

    let found = ConnectTarget::find(&targets, "1234").expect("realm should match by id");

    assert_eq!(found.name, "My Realm");
    assert_eq!(found.kind, ConnectTargetKind::Realm);
}

#[test]
fn find_returns_none_for_an_unknown_id() {
    assert!(ConnectTarget::find(&[], "proxy-a").is_none());
}

// A proxy entry without a host is unusable, and connecting to it would start a listener
// pointed at nothing. Reporting it as unusable beats a proxy that no Bedrock client can reach.
#[test]
fn proxy_without_host_is_not_connectable() {
    assert!(!proxy("proxy-a", "Broken", None, Some(19132)).is_connectable());
}

#[test]
fn proxy_without_port_is_not_connectable() {
    assert!(!proxy("proxy-a", "Broken", Some("127.0.0.1"), None).is_connectable());
}

#[test]
fn proxy_with_host_and_port_is_connectable() {
    assert!(proxy("proxy-a", "Local BDS", Some("127.0.0.1"), Some(19132)).is_connectable());
}

// A realm's address is resolved from Xbox Live at connect time, so it never carries one here
// and must not be judged unusable for the absence.
#[test]
fn realm_is_connectable_without_host_or_port() {
    assert!(realm("1234", "My Realm").is_connectable());
}
