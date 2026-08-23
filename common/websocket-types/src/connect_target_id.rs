use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Where a connectable world came from.
///
/// Rides in the id itself rather than beside it, so a controller quotes back one opaque
/// string and the client can route it without consulting the list a second time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum ConnectTargetSource {
    Saved,
    Server,
    Realm,
}

impl ConnectTargetSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Saved => "saved",
            Self::Server => "server",
            Self::Realm => "realm",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "saved" => Some(Self::Saved),
            "server" => Some(Self::Server),
            "realm" => Some(Self::Realm),
            _ => None,
        }
    }
}

/// A target id as it travels on the wire: `{source}:{native}`.
///
/// Splitting on the first colon and never a later one is load-bearing: a `server` entry's
/// native half is `host:port`, and any other split hands the connect path a truncated
/// hostname that resolves to nothing.
pub struct ConnectTargetId;

impl ConnectTargetId {
    pub fn mint(source: ConnectTargetSource, native: &str) -> String {
        format!("{}:{}", source.as_str(), native)
    }

    pub fn parse(id: &str) -> Option<(ConnectTargetSource, &str)> {
        let (source, native) = id.split_once(':')?;
        if native.is_empty() {
            return None;
        }
        Some((ConnectTargetSource::parse(source)?, native))
    }
}
