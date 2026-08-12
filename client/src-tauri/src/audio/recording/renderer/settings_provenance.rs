/// Which of the three sources supplied the spatial settings a render ran under.
///
/// Carried so a log line can say why an export sounds like it does. A render under `Defaults` on
/// a server that changed its broadcast range is correct code producing the wrong curve, and
/// nothing else in the output would show it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsProvenance {
    LiveSession,
    LastKnown,
    Defaults,
}

impl SettingsProvenance {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::LiveSession => "live session",
            Self::LastKnown => "last known",
            Self::Defaults => "defaults",
        }
    }
}
