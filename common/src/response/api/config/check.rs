use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::{ApiConfigResponse, ProtocolCompatibility};

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct ApiConfigCheckResponse {
    pub config: ApiConfigResponse,
    pub client_version: String,
    pub compatible: bool,
    pub client_too_old: bool,
}

impl ApiConfigCheckResponse {
    // The comparison itself lives on ProtocolCompatibility, so the pre-auth server
    // check reaches this same verdict from the same rule rather than a copy of it.
    pub fn from_config(config: ApiConfigResponse, client_version: &str) -> Self {
        let compatibility =
            ProtocolCompatibility::between(&config.protocol_version, client_version);

        Self {
            config,
            client_version: compatibility.client_version,
            compatible: compatibility.compatible,
            client_too_old: compatibility.client_too_old,
        }
    }
}
