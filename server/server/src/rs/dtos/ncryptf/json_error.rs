//! JSON error container for API responses

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Error container for JSON responses
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JsonError {
    /// Error map of key-value pairs
    pub errors: HashMap<String, String>,
}

impl JsonError {
    /// Creates a new empty JsonError
    pub fn new() -> Self {
        Self {
            errors: HashMap::new(),
        }
    }

    /// Add an error to the error map
    pub fn add_error(&self, key: &str, value: &str) -> Self {
        let mut errors = self.errors.clone();
        errors.insert(key.to_string(), value.to_string());
        Self { errors }
    }
}

impl Default for JsonError {
    fn default() -> Self {
        Self::new()
    }
}
