use serde::Deserialize;

use super::FlagsmithFlag;

#[derive(Debug, Deserialize)]
pub(crate) struct FlagsmithIdentityResponse {
    pub flags: Vec<FlagsmithFlag>,
}
