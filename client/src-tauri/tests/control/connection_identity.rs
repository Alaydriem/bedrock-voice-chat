use bvc_client_lib::control::ConnectionIdentity;

// The reporter resends every preference once per connection, and the server enforces
// crouch-to-whisper from what it was sent. A reconnect under the same identity must therefore
// read as a new connection; compared by name alone it does not, and the server that just evicted
// the old connection's preferences is not sent them again until the next resync.
#[test]
fn a_reconnect_under_the_same_identity_is_a_new_connection() {
    let identity = ConnectionIdentity::new();
    identity.set(Some("minecraft:Alice".to_string()));
    let first = identity.generation();

    identity.set(None);
    identity.set(Some("minecraft:Alice".to_string()));

    assert_ne!(identity.generation(), first);
    assert_eq!(identity.get().as_deref(), Some("minecraft:Alice"));
}

#[test]
fn reading_the_identity_does_not_start_a_new_connection() {
    let identity = ConnectionIdentity::new();
    identity.set(Some("minecraft:Alice".to_string()));
    let first = identity.generation();

    let _ = identity.get();

    assert_eq!(identity.generation(), first);
}
