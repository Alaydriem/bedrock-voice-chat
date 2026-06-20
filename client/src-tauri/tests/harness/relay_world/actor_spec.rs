use super::Srv;

/// Placement of one actor: its name, the voice server it connects to, and the
/// index of the realm (fake upstream) its proxy points at.
pub struct ActorSpec<'a> {
    pub name: &'a str,
    pub server: Srv,
    pub realm: usize,
}
