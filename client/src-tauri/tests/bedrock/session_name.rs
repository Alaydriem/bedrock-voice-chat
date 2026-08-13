use bvc_client_lib::bedrock::SessionName;

// The Friends-tab entry is the only name the user sees, so it must be the name
// they chose — never a branded string they never typed.
#[test]
fn a_resolved_entry_uses_its_own_name() {
    assert_eq!(
        SessionName::world(Some("Truly Bedrock SMP"), "tbs7.nodecraft.gg"),
        "Truly Bedrock SMP"
    );
}

// A typed address nothing names is still the name the user chose.
#[test]
fn an_unresolved_target_falls_back_to_the_host() {
    assert_eq!(
        SessionName::world(None, "play.example.com"),
        "play.example.com"
    );
}

#[test]
fn a_blank_resolved_name_is_treated_as_unresolved() {
    assert_eq!(
        SessionName::world(Some("   "), "play.example.com"),
        "play.example.com"
    );
}

// Two people on the same LAN in the same world would otherwise advertise
// byte-identical entries and neither could tell which was theirs.
#[test]
fn the_second_line_names_the_player() {
    assert_eq!(
        SessionName::owner(Some("Alaydriem")),
        "Alaydriem · Bedrock Voice Chat"
    );
}

// Signed out, or a keyring read that failed: brand it and move on rather than
// advertising a stray separator.
#[test]
fn the_second_line_is_branding_alone_when_the_player_is_unknown() {
    assert_eq!(SessionName::owner(None), "Bedrock Voice Chat");
    assert_eq!(SessionName::owner(Some("  ")), "Bedrock Voice Chat");
}
