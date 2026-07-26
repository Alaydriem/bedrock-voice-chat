use schemars::JsonSchema;
use serde::Serialize;

/// Per-component readiness snapshot returned by /health/readiness. Each
/// component is "ok" or "down".
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct ReadinessResponse {
    pub database: String,
    pub quic: String,
    pub certificate: String,
}

impl ReadinessResponse {
    pub fn ready(&self) -> bool {
        self.database == "ok" && self.quic == "ok" && self.certificate == "ok"
    }
}
