/// How strictly a client checks the certificate a server presents.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerifierMode {
    /// The play-zone rules plus ordinary validation against the public and server roots.
    Strict,
    /// Accepts any certificate chain; handshake signatures are still checked. This is what
    /// a debug build uses against development servers whose certificates no root signed.
    /// A release build treats it as `Strict`.
    Permissive,
}

impl VerifierMode {
    /// The mode every client surface uses unless a caller pins one: `Permissive` in a debug
    /// build, which must reach development servers and the end-to-end harness, and `Strict`
    /// in a release build.
    pub const fn for_build() -> Self {
        if cfg!(debug_assertions) {
            Self::Permissive
        } else {
            Self::Strict
        }
    }
}
