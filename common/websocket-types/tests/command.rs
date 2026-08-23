use websocket_types::Command;

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
