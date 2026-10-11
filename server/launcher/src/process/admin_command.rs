use super::AdminVerb;

/// `bvc admin grant|revoke <Player>` typed into the panel console.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdminCommand {
    pub verb: AdminVerb,
    pub player: String,
}

impl AdminCommand {
    /// The `bvc-server` arguments after `-c <config>`.
    pub fn args(&self) -> [String; 4] {
        [
            "admin".to_string(),
            self.verb.as_str().to_string(),
            "-p".to_string(),
            self.player.clone(),
        ]
    }

    pub fn label(&self) -> String {
        format!("bvc admin {} {}", self.verb.as_str(), self.player)
    }
}
