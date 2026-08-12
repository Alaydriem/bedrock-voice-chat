use std::collections::HashSet;

/// Who has been heard in a session so far, split the way the manifest stores it.
///
/// Both the live path and the shutdown drain classify emitters, and both used to do it
/// with their own copy of the same match. One type means the input path cannot quietly
/// skip the step, which is how the recorder came to never name the person recording.
pub struct ParticipantIndex {
    players: HashSet<String>,
    jukebox: HashSet<String>,
}

impl ParticipantIndex {
    pub fn new() -> Self {
        Self {
            players: HashSet::new(),
            jukebox: HashSet::new(),
        }
    }

    /// True when this name had not been heard before, which is when the manifest on disk
    /// has fallen behind.
    pub fn observe(&mut self, emitter: &str) -> bool {
        let target = if emitter.starts_with(common::consts::audio::JUKEBOX_PLAYER_PREFIX) {
            &mut self.jukebox
        } else {
            &mut self.players
        };
        target.insert(emitter.to_string())
    }

    pub fn players(&self) -> Vec<String> {
        self.players.iter().cloned().collect()
    }

    pub fn jukebox(&self) -> Vec<String> {
        self.jukebox.iter().cloned().collect()
    }
}

impl Default for ParticipantIndex {
    fn default() -> Self {
        Self::new()
    }
}
