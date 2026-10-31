/// A game server's statement that the name it reported for a player is an alias of the
/// gamertag that player signs in with.
///
/// Made when a roster entry carries `alternative_identity`. Recorded only after the gamertag
/// is registered, and only when the platform UUID shows the alias is not someone else's.
#[derive(Debug, Clone, PartialEq)]
pub struct AliasClaim {
    pub alias: String,
    pub gamertag: String,
    pub player_uuid: Option<String>,
}

impl AliasClaim {
    pub fn new(alias: &str, gamertag: &str, player_uuid: Option<&str>) -> Self {
        Self {
            alias: alias.to_string(),
            gamertag: gamertag.to_string(),
            player_uuid: player_uuid.map(String::from),
        }
    }
}
