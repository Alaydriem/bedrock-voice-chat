use bvc_client_lib::bedrock::MinecraftTranslation;

fn p(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| s.to_string()).collect()
}

// `/say hello` on the server. It arrives as a translation key, not as text, which is why it
// was invisible before this rendered it.
#[test]
fn a_server_say_renders_with_its_sender() {
    let out = MinecraftTranslation::render("chat.type.announcement", &p(&["Server", "hello"]))
        .expect("announcements must render");
    assert_eq!(out, "[Server] hello");
}

#[test]
fn a_join_renders() {
    let out = MinecraftTranslation::render("multiplayer.player.joined", &p(&["Petra"]))
        .expect("joins must render");
    assert_eq!(out, "Petra joined the game");
}

#[test]
fn a_leave_renders() {
    let out = MinecraftTranslation::render("multiplayer.player.left", &p(&["Petra"]))
        .expect("leaves must render");
    assert_eq!(out, "Petra left the game");
}

#[test]
fn a_death_with_a_killer_names_both() {
    let out = MinecraftTranslation::render("death.attack.mob", &p(&["Moth", "Enderman"]))
        .expect("deaths must render");
    assert_eq!(out, "Moth was slain by Enderman");
}

#[test]
fn a_positional_template_respects_its_indices() {
    let out = MinecraftTranslation::render("death.attack.arrow", &p(&["Moth", "Skeleton"]))
        .expect("deaths must render");
    assert_eq!(out, "Moth was shot by Skeleton");
}

// Mojang's death catalogue keeps growing, so the table will always trail the game. The
// parameter count still carries the important part.
#[test]
fn an_unknown_solo_death_still_reports_a_death() {
    let out = MinecraftTranslation::render("death.attack.some_new_hazard_2027", &p(&["Moth"]))
        .expect("any death must render something");
    assert_eq!(out, "Moth died");
}

// A second parameter is the killer. Naming them is the difference between a useful line and
// a shrug — "AlayCamera died" loses the only interesting part of the event.
#[test]
fn an_unknown_death_with_a_killer_names_the_killer() {
    let out = MinecraftTranslation::render(
        "death.attack.some_new_weapon_2027",
        &p(&["AlayCamera", "Alaydriem"]),
    )
    .expect("any death must render something");
    assert_eq!(out, "AlayCamera was killed by Alaydriem");
}

#[test]
fn a_player_kill_with_a_weapon_names_the_weapon() {
    let out = MinecraftTranslation::render(
        "death.attack.player.item",
        &p(&["AlayCamera", "Alaydriem", "Netherite Sword"]),
    )
    .expect("player kills must render");
    assert_eq!(out, "AlayCamera was slain by Alaydriem using Netherite Sword");
}

// Everything else — achievements, command feedback, UI text — is dropped rather than relayed.
// Chat is for what people say, not for every string the game emits.
#[test]
fn an_unrelated_key_is_not_rendered() {
    assert!(MinecraftTranslation::render("commands.gamemode.success", &p(&["x"])).is_none());
}

// The wire is inconsistent about whether the key carries a leading percent.
#[test]
fn a_percent_prefixed_key_still_resolves() {
    let out = MinecraftTranslation::render("%multiplayer.player.joined", &p(&["Petra"]))
        .expect("a prefixed key must resolve");
    assert_eq!(out, "Petra joined the game");
}

#[test]
fn a_missing_parameter_leaves_no_placeholder_behind() {
    let out = MinecraftTranslation::render("death.attack.mob", &p(&["Moth"]))
        .expect("deaths must render");
    assert!(!out.contains('%'), "no placeholder should survive: {out}");
}
