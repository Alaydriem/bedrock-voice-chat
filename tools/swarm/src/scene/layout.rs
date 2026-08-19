use crate::scene::Placement;

/// One scenario resolved against a scene file: everybody who appears, and how.
///
/// The two lists are populated by different mechanisms and cannot be interchanged.
/// `staged` players exist only in the server's position cache, which is enough to be
/// seen and heard-of but never to be a group member — membership is written by an
/// authenticated client for itself. `connected` players are real processes, which is
/// what a group costs.
pub struct SceneLayout {
    pub staged: Vec<Placement>,
    pub connected: Vec<String>,
    pub group_name: Option<String>,
}

impl SceneLayout {
    pub fn staged_only(staged: Vec<Placement>) -> Self {
        Self {
            staged,
            connected: Vec::new(),
            group_name: None,
        }
    }

    pub fn with_group(staged: Vec<Placement>, connected: Vec<String>, group_name: String) -> Self {
        Self {
            staged,
            connected,
            group_name: Some(group_name),
        }
    }

    pub fn needs_clients(&self) -> bool {
        !self.connected.is_empty()
    }
}
