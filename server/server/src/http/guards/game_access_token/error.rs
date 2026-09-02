#[derive(Debug)]
pub enum GameAccessTokenError {
    Missing,
    Invalid,
    /// This deployment has issued no credential of either kind, so nothing could have
    /// matched. Distinct from `Invalid` because the fix is to mint a token, not to correct
    /// the one a mod is sending.
    NotConfigured,
}
