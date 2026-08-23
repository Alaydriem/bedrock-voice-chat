// What Minecraft's Friends / LAN tab renders for a BVC session: the RakNet
// advertisement's `motd` on the first line and `sub_motd` on the second.
//
// The user picked the world, so the world is what they must see. Branding goes on
// the second line beside the player's name — two clients on the same LAN in the
// same world would otherwise advertise identical entries with nothing to tell
// them apart.
pub struct SessionName;

impl SessionName {
    // First line. A target nothing names falls back to its host, which is still
    // the name the user typed.
    pub fn world(resolved: Option<&str>, host: &str) -> String {
        match resolved.map(str::trim) {
            Some(name) if !name.is_empty() => name.to_string(),
            _ => host.to_string(),
        }
    }

    // Second line. Signed out, or a credential read that failed, brands the entry
    // alone rather than advertising a dangling separator.
    pub fn owner(gamertag: Option<&str>) -> String {
        match gamertag.map(str::trim) {
            Some(tag) if !tag.is_empty() => format!("{tag} · Bedrock Voice Chat"),
            _ => "Bedrock Voice Chat".to_string(),
        }
    }
}
