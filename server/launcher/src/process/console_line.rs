use super::{AdminCommand, AdminVerb};

/// What the launcher does with one line typed into the panel console.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConsoleLine {
    Forward,
    Admin(AdminCommand),
    Usage,
}

impl ConsoleLine {
    pub const USAGE: &'static str =
        "Usage: bvc admin grant <Player> | bvc admin revoke <Player>";

    pub fn parse(line: &[u8]) -> Self {
        let text = String::from_utf8_lossy(line);
        let mut words = text.trim().splitn(2, char::is_whitespace);
        if words.next() != Some("bvc") {
            return Self::Forward;
        }
        let rest = words.next().unwrap_or("").trim_start();
        let mut parts = rest.splitn(3, char::is_whitespace);
        let (Some("admin"), Some(verb), Some(player)) = (parts.next(), parts.next(), parts.next())
        else {
            return Self::Usage;
        };
        let verb = match verb {
            "grant" => AdminVerb::Grant,
            "revoke" => AdminVerb::Revoke,
            _ => return Self::Usage,
        };
        let player = player.trim();
        if player.is_empty() {
            return Self::Usage;
        }
        Self::Admin(AdminCommand {
            verb,
            player: player.to_string(),
        })
    }
}
