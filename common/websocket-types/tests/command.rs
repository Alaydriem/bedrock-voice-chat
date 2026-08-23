use websocket_types::{Command, CommandMessage, DeviceType};

// A toggle rather than a setter, matching `mute`: a controller button has no way to read the
// current state before it is pressed, so a setter would need one and a button cannot supply it.
#[test]
fn jukebox_parses_with_no_argument() {
    let cmd = Command::from_json(r#"{"action":"jukebox"}"#).expect("jukebox parses");

    assert!(matches!(cmd, Command::Jukebox));
}

// A level rather than a toggle: a dial or a slider sends an absolute value and, unlike a button,
// knows what it is asking for. Percent rather than a fraction, matching the panel's own slider and
// every other level a controller sends.
#[test]
fn jukebox_volume_parses_with_its_level() {
    match Command::from_json(r#"{"action":"jukeboxvolume","level":150}"#)
        .expect("jukeboxvolume parses")
    {
        Command::JukeboxVolume { level } => assert_eq!(level, 150),
        other => panic!("expected JukeboxVolume, got {other:?}"),
    }
}

#[test]
fn group_commands_parse() {
    match Command::from_json(r#"{"action":"creategroup","name":"Ops"}"#).expect("creategroup") {
        Command::CreateGroup { name } => assert_eq!(name, "Ops"),
        other => panic!("expected CreateGroup, got {other:?}"),
    }

    match Command::from_json(r#"{"action":"joingroup","name":"Ops"}"#).expect("joingroup") {
        Command::JoinGroup { name } => assert_eq!(name, "Ops"),
        other => panic!("expected JoinGroup, got {other:?}"),
    }

    // Names nothing, so a controller that lost track of which group it is in can send it
    // without first asking.
    let leave = Command::from_json(r#"{"action":"leavegroup"}"#).expect("leavegroup");
    assert!(matches!(leave, Command::LeaveGroup));
}

// A group name is not optional on the two that need one. Accepting a missing name would create or
// join a group called the empty string, which no picker can show and nobody can leave by name.
#[test]
fn a_group_command_without_a_name_is_refused() {
    assert!(Command::from_json(r#"{"action":"creategroup"}"#).is_err());
    assert!(Command::from_json(r#"{"action":"joingroup"}"#).is_err());
}

// The three that carry nothing. `action` alone is the whole message, so a controller sends the
// same bytes every time and the client does not have to tolerate an absent argument.
#[test]
fn the_argumentless_commands_parse() {
    assert!(matches!(
        Command::from_json(r#"{"action":"ping"}"#).expect("ping"),
        Command::Ping
    ));
    assert!(matches!(
        Command::from_json(r#"{"action":"record"}"#).expect("record"),
        Command::Record
    ));
    assert!(matches!(
        Command::from_json(r#"{"action":"state"}"#).expect("state"),
        Command::State
    ));
}

// `down` is the press, not a toggle. A controller holding the mic open owns the release, so the
// press and the release are the same action carrying different values rather than two actions.
#[test]
fn ptt_carries_the_press_state() {
    match Command::from_json(r#"{"action":"ptt","down":true}"#).expect("ptt parses") {
        Command::Ptt { down } => assert!(down),
        other => panic!("expected Ptt, got {other:?}"),
    }

    match Command::from_json(r#"{"action":"ptt","down":false}"#).expect("ptt parses") {
        Command::Ptt { down } => assert!(!down),
        other => panic!("expected Ptt, got {other:?}"),
    }
}

// Deafen is not its own action. It is a mute naming the output device, which is what keeps one
// action covering both directions instead of two that can disagree.
#[test]
fn mute_names_the_device() {
    match Command::from_json(r#"{"action":"mute","device":"input"}"#).expect("input mute") {
        Command::Mute { device } => assert!(matches!(device, DeviceType::Input)),
        other => panic!("expected Mute, got {other:?}"),
    }

    match Command::from_json(r#"{"action":"mute","device":"output"}"#).expect("output mute") {
        Command::Mute { device } => assert!(matches!(device, DeviceType::Output)),
        other => panic!("expected Mute, got {other:?}"),
    }
}

// `key` sits beside `action` rather than inside it, so it is read by a second pass over the same
// text. The command must parse identically whether or not the key is there — a controller that
// authenticates and one that does not send the same command shape.
#[test]
fn a_command_message_carries_an_optional_key() {
    let with_key =
        CommandMessage::from_json(r#"{"action":"ping","key":"secret123"}"#).expect("keyed");
    assert!(matches!(with_key.command, Command::Ping));
    assert_eq!(with_key.key.as_deref(), Some("secret123"));

    let without_key = CommandMessage::from_json(r#"{"action":"ping"}"#).expect("unkeyed");
    assert!(matches!(without_key.command, Command::Ping));
    assert_eq!(without_key.key, None);
}
