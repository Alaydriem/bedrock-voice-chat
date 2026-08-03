use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// A sign-in code, and nothing else.
///
/// The code is the credential and the identifier both: its row carries the player it was
/// issued for, so the server resolves the gamertag and the game without being told
/// either. A gamertag alongside it would carry no secret — gamertags are public — and
/// could only disagree with what the code already says.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[cfg_attr(feature = "openapi", derive(schemars::JsonSchema))]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct CodeLoginRequest {
    pub code: String,
}
