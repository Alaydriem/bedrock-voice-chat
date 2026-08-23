use websocket_types::{Command, CommandMessage};

// A controller that lost track of the client sends this without asking what is running
// first, so parsing must not require a target it does not have.
#[test]
fn parses_a_disconnect_that_names_no_target() {
    let cmd = Command::from_json(r#"{"action":"disconnect"}"#).expect("disconnect should parse");
    assert!(matches!(cmd, Command::Disconnect));
}

#[test]
fn parses_a_disconnect_carrying_the_auth_key() {
    let msg = CommandMessage::from_json(r#"{"action":"disconnect","key":"secret123"}"#)
        .expect("disconnect with a key should parse");

    assert!(matches!(msg.command, Command::Disconnect));
    assert_eq!(msg.key, Some("secret123".to_string()));
}
