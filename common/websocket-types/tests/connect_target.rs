use websocket_types::{ConnectTarget, ConnectTargetKind};

fn target(id: &str, name: &str, kind: ConnectTargetKind) -> ConnectTarget {
    ConnectTarget::new(id.to_string(), name.to_string(), kind)
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

// The constructor is the only way a glyph is attached, so no construction site can forget one.
// There are two of them and they live in different crates.
#[test]
fn a_target_carries_the_glyph_derived_from_its_name() {
    let target = target("realm:1234", "My Realm", ConnectTargetKind::Realm);

    assert_eq!(target.glyph, websocket_types::ServerGlyph::of("My Realm"));
}

// Two worlds a picker shows side by side have to be distinguishable, which is the whole reason
// the glyph travels.
#[test]
fn two_names_get_two_glyphs() {
    let a = target("realm:1", "Ops", ConnectTargetKind::Realm);
    let b = target("realm:2", "a", ConnectTargetKind::Realm);

    assert_ne!(a.glyph, b.glyph);
}

// The id has no bearing on the tile. Two saved entries for the same world under different ids
// must not read as two different places.
#[test]
fn the_glyph_follows_the_name_and_not_the_id() {
    let saved = target("saved:one", "Hearthhold", ConnectTargetKind::Proxy);
    let discovered = target("server:hearthhold.net:19132", "Hearthhold", ConnectTargetKind::Proxy);

    assert_eq!(saved.glyph, discovered.glyph);
}
