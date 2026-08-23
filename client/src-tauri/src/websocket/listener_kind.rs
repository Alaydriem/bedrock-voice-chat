/// Which listener a connection arrived on.
///
/// The two are not interchangeable and the difference has to travel with the connection: the
/// internal listener authenticates a per-process token and serves one push route, while the
/// external listener authenticates the user's key and serves the command protocol a third-party
/// integration speaks. Deciding that from the socket alone is not possible once the streams have
/// been split, so it is carried explicitly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListenerKind {
    Internal,
    External,
}

impl ListenerKind {
    /// Whether a connection here belongs in the table the settings pane lists.
    ///
    /// The app's own connection does not. Excluding it here rather than filtering the snapshot
    /// means there is no list that ever contains it.
    pub fn registers_clients(&self) -> bool {
        matches!(self, Self::External)
    }
}
