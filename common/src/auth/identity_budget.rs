use std::future::Future;

use crate::auth::AuthError;
use crate::net::NetTimeouts;

/// Caps a whole identity provider exchange.
///
/// The exchange visits five provider hosts in series. Bounding each leg alone permits five
/// times the per-leg budget, which is longer than any client waits, so the sum needs a cap of
/// its own.
pub struct IdentityBudget;

impl IdentityBudget {
    pub async fn enforce<F, T>(exchange: F) -> Result<T, AuthError>
    where
        F: Future<Output = Result<T, AuthError>>,
    {
        match tokio::time::timeout(NetTimeouts::IDENTITY, exchange).await {
            Ok(result) => result,
            // Network rather than a refusal: nothing here reached a decision about the
            // account, and the route turns a refusal into a status that tells the player to
            // sign in again instead of to try later.
            Err(_) => Err(AuthError::Network(format!(
                "the identity provider exchange exceeded {:?}",
                NetTimeouts::IDENTITY
            ))),
        }
    }
}
