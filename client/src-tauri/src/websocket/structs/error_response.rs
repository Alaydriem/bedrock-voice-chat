use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

/// Error response wrapper
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ErrorResponse {
    pub success: bool,
    pub error: String,
}

impl ErrorResponse {
    pub fn new(error: String) -> Self {
        Self {
            success: false,
            error,
        }
    }
}
