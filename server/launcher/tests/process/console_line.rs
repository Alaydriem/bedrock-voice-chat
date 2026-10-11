use bvc_launcher::process::{AdminCommand, AdminVerb, ConsoleLine};

fn admin(verb: AdminVerb, player: &str) -> ConsoleLine {
    ConsoleLine::Admin(AdminCommand {
        verb,
        player: player.to_string(),
    })
}

#[test]
fn bds_commands_are_forwarded() {
    assert_eq!(ConsoleLine::parse(b"list\n"), ConsoleLine::Forward);
    assert_eq!(ConsoleLine::parse(b"bvcx admin grant A\n"), ConsoleLine::Forward);
    assert_eq!(ConsoleLine::parse(b"\n"), ConsoleLine::Forward);
}

#[test]
fn grant_and_revoke_are_parsed() {
    assert_eq!(
        ConsoleLine::parse(b"bvc admin grant Alaydriem\n"),
        admin(AdminVerb::Grant, "Alaydriem")
    );
    assert_eq!(
        ConsoleLine::parse(b"  bvc admin revoke Alaydriem \r\n"),
        admin(AdminVerb::Revoke, "Alaydriem")
    );
}

#[test]
fn a_name_with_spaces_is_kept_whole() {
    assert_eq!(
        ConsoleLine::parse(b"bvc admin grant Some Player\n"),
        admin(AdminVerb::Grant, "Some Player")
    );
}

#[test]
fn anything_else_starting_with_bvc_is_usage() {
    let lines: [&[u8]; 6] = [
        b"bvc\n",
        b"bvc admin\n",
        b"bvc admin grant\n",
        b"bvc admin grant   \n",
        b"bvc token mint\n",
        b"bvc admin deny A\n",
    ];
    for line in lines {
        assert_eq!(
            ConsoleLine::parse(line),
            ConsoleLine::Usage,
            "{:?}",
            String::from_utf8_lossy(line)
        );
    }
}

#[test]
fn the_cli_arguments_name_the_verb_and_player() {
    let command = AdminCommand {
        verb: AdminVerb::Revoke,
        player: "Some Player".to_string(),
    };

    assert_eq!(
        command.args(),
        ["admin", "revoke", "-p", "Some Player"].map(String::from)
    );
    assert_eq!(command.label(), "bvc admin revoke Some Player");
}
