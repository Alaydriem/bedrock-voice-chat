use serde::Deserialize;

use super::Xui;

/// Display claims containing user info
#[derive(Deserialize)]
pub(crate) struct DisplayClaims {
    pub xui: Vec<Xui>,
}
