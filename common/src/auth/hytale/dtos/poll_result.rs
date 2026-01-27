//! Poll result for device flow

use crate::auth::provider::AuthResult;

/// Result of polling for device flow completion
#[derive(Debug, Clone)]
pub enum PollResult {
    /// Authorization is pending - user hasn't completed yet
    Pending,

    /// Polling too fast - slow down
    SlowDown,

    /// Successfully authenticated
    Success(AuthResult),

    /// Device code has expired
    Expired,

    /// User denied the authorization
    Denied,
}
