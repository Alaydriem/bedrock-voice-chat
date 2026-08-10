use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::structs::metrics::TransportKind;
use crate::structs::reachability::AddressFamilyPreference;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct SessionDiagnostics {
    pub server: Option<String>,
    pub protocol_version: Option<String>,
    pub proximity_range: Option<f32>,
    pub falloff: Option<String>,
    // The reachability probe's verdict, which distinguishes "preferred IPv6 and IPv6 carried
    // the session" from "preferred IPv6, IPv4 carried it".
    pub family_preference: Option<AddressFamilyPreference>,
    // Which transport carried this session. Absent when nothing is connected. Without it a
    // latency or loss figure cannot be compared against another, because the two transports
    // fail and recover in different ways.
    pub transport: Option<TransportKind>,
}
