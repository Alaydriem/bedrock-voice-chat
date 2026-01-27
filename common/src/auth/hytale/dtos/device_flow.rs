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
    pub(crate) fn new(
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_flow_device_code_visibility() {
        let flow = DeviceFlow {
            device_code: "secret".to_string(),
            user_code: "ABCD-1234".to_string(),
            verification_uri: "https://example.com/device".to_string(),
            verification_uri_complete: "https://example.com/device?code=ABCD-1234".to_string(),
            expires_in: 600,
            interval: 5,
        };

        // device_code is accessible via method
        assert_eq!(flow.device_code(), "secret");

        // user_code is public
        assert_eq!(flow.user_code, "ABCD-1234");
    }

    #[test]
    fn test_device_flow_serialization_hides_device_code() {
        let flow = DeviceFlow {
            device_code: "secret".to_string(),
            user_code: "ABCD-1234".to_string(),
            verification_uri: "https://example.com/device".to_string(),
            verification_uri_complete: "https://example.com/device?code=ABCD-1234".to_string(),
            expires_in: 600,
            interval: 5,
        };

        let json = serde_json::to_string(&flow).unwrap();

        // device_code should not appear in serialized output
        assert!(!json.contains("secret"));
        assert!(json.contains("ABCD-1234"));
    }
}
