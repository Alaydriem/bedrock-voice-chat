use bvc_client_lib::websocket::ListenerKind;

// The app's own connection must not appear in the table the settings pane lists, and the
// reason is not cosmetic: that table is user-visible, and a row for the internal channel
// would advertise that a second listener exists and invite someone to point a plugin at it.
#[test]
fn internal_connections_are_not_registered() {
    assert!(!ListenerKind::Internal.registers_clients());
    assert!(ListenerKind::External.registers_clients());
}
