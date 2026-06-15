//! Device flow session data

use serde::{Deserialize, Serialize};

/// An active device code flow session
///
/// Contains the information needed to:
/// - Display to the user (user_code, verification URLs)
/// - Poll for completion (device_code - crate visibility)
/// - Manage timing (expires_in, interval)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceFlow {
    /// The device code (used internally for polling)
    #[serde(skip_serializing)]
    pub(crate) device_code: String,

    /// The code to display to the user
    pub user_code: String,

    /// The URL where the user enters the code
    pub verification_uri: String,

    /// The URL with the code pre-filled (convenience for users)
    pub verification_uri_complete: String,

    /// How long until the device code expires (seconds)
    pub expires_in: u64,

    /// Minimum interval between poll requests (seconds)
    pub interval: u64,
}

impl DeviceFlow {
    /// Create a new DeviceFlow
    pub fn new(
        device_code: String,
        user_code: String,
        verification_uri: String,
        verification_uri_complete: String,
        expires_in: u64,
        interval: u64,
    ) -> Self {
        Self {
            device_code,
            user_code,
            verification_uri,
            verification_uri_complete,
            expires_in,
            interval,
        }
    }

    /// Get the device code for session storage
    pub fn device_code(&self) -> &str {
        &self.device_code
    }
}
