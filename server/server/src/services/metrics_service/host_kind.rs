// What kind of process hosts this server, reported on every telemetry event.
//
// The vocabulary is closed. The value comes from an environment variable or a
// mod's FFI call, and an unrecognised string must not become an unbounded
// property value in the metrics pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostKind {
    Standalone,
    Docker,
    Paper,
    Fabric,
    PowerNukkitX,
    Unknown,
}

impl HostKind {
    const ENV_VAR: &'static str = "BVC_HOST";

    // Empty is a standalone binary; a value outside the vocabulary is Unknown.
    pub fn parse(raw: &str) -> HostKind {
        match raw.trim().to_ascii_lowercase().as_str() {
            "" | "standalone" => HostKind::Standalone,
            "docker" => HostKind::Docker,
            "paper" => HostKind::Paper,
            "fabric" => HostKind::Fabric,
            "powernukkitx" => HostKind::PowerNukkitX,
            _ => HostKind::Unknown,
        }
    }

    // An unset variable is a standalone binary.
    pub fn from_env() -> HostKind {
        match std::env::var(Self::ENV_VAR) {
            Ok(raw) => Self::parse(&raw),
            Err(std::env::VarError::NotPresent) => HostKind::Standalone,
            Err(std::env::VarError::NotUnicode(_)) => HostKind::Unknown,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            HostKind::Standalone => "standalone",
            HostKind::Docker => "docker",
            HostKind::Paper => "paper",
            HostKind::Fabric => "fabric",
            HostKind::PowerNukkitX => "powernukkitx",
            HostKind::Unknown => "unknown",
        }
    }
}
