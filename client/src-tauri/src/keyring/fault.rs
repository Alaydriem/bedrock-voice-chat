/// Which of the two credential-write failures happened, and the fault code that reports it.
///
/// Derived from the rendered message rather than from a typed error. The platform error arrives
/// as a `String` through three layers - `dbus-secret-service`, the `keyring-core` store, and
/// `tauri-plugin-keyring` - and none of them preserve the originating variant. The messages this
/// matches are pinned in `tests/keyring/fault.rs` so an upstream rewording fails a test instead
/// of silently reclassifying every Linux write failure.
pub struct KeyringFault;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyringFaultKind {
    /// The OS keyring exists but cannot accept a write until someone acts on it: no collection to
    /// write into, a locked collection, or a dismissed unlock prompt. On Linux this is the state a
    /// machine is left in when `libpam-gnome-keyring` never created `login.keyring`, which is what
    /// automatic login produces.
    Unusable,
    /// Anything else. Reported generically, because the remedy is not knowable from here.
    Other,
}

impl KeyringFault {
    /// The `dbus_secret_service::Error` messages that mean the keyring needs attention.
    ///
    /// `Unavailable` is deliberately absent. The store connects to the session bus eagerly when
    /// the plugin initialises, so a missing bus or provider aborts the launch and can never reach
    /// a credential write.
    const UNUSABLE: [&'static str; 3] = [
        "no result found",
        "object locked",
        "unlock prompt was dismissed",
    ];

    pub fn classify(message: &str) -> KeyringFaultKind {
        let lowered = message.to_ascii_lowercase();

        if Self::UNUSABLE.iter().any(|m| lowered.contains(m)) {
            return KeyringFaultKind::Unusable;
        }

        KeyringFaultKind::Other
    }

    pub fn code(kind: KeyringFaultKind) -> &'static str {
        match kind {
            KeyringFaultKind::Unusable => "AUTH04",
            KeyringFaultKind::Other => "AUTH03",
        }
    }

    pub fn label(message: &str) -> String {
        Self::code(Self::classify(message)).to_string()
    }
}
