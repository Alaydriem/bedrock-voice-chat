use websocket_types::{ConnectTarget, ConnectTargetKind};

fn target(id: &str, name: &str, kind: ConnectTargetKind) -> ConnectTarget {
    ConnectTarget {
        id: id.to_string(),
        name: name.to_string(),
        kind,
    }
}

// A list and a connect are two calls, so the list is a snapshot the caller acts on later.
// Matching is by id and never by list position, or an entry added between the two calls
// connects the operator to the wrong world.
#[test]
fn find_matches_on_id_not_position() {
    let targets = vec![
        target("saved:proxy-a", "Local BDS", ConnectTargetKind::Proxy),
        target("realm:1234", "My Realm", ConnectTargetKind::Realm),
    ];

    let found = ConnectTarget::find(&targets, "realm:1234").expect("realm should match by id");

    assert_eq!(found.name, "My Realm");
    assert_eq!(found.kind, ConnectTargetKind::Realm);
}

#[test]
fn find_returns_none_for_an_unknown_id() {
    assert!(ConnectTarget::find(&[], "saved:proxy-a").is_none());
}
