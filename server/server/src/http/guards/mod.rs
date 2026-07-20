mod access_token;
mod access_token_error;
//pub(crate) use access_token::AccessToken;
pub(crate) use access_token_error::AccessTokenError;

mod admin;
mod admin_guard_error;
pub(crate) use admin::AdminGuard;
pub(crate) use admin_guard_error::AdminGuardError;

mod hytale_session_id;
pub(crate) use hytale_session_id::HytaleSessionId;

mod mc_access_token;
mod mc_access_token_error;
pub(crate) use mc_access_token::MCAccessToken;
pub(crate) use mc_access_token_error::MCAccessTokenError;

mod original_filename;
pub(crate) use original_filename::OriginalFilename;

// Per-IP quota shared by the two unauthenticated relay code endpoints. Sized
// ABOVE the relay's own legitimate cadence, not just against brute force: the
// orchestrator re-offers every 15s while a link is unproven (~4 offer+redeem
// pairs/min per dialing peer), and several peers can share one client IP
// (co-hosted servers, NAT — and every e2e harness). The previous 3/min sat
// BELOW that cadence, so mesh formation throttled itself into 20s governor
// waits. Codes are single-use, short-TTL and high-entropy, so the limiter is a
// resource bound, not the security boundary.
pub const RELAY_CODE_RATE_PER_MINUTE: u32 = 30;

mod relay_offer_rate_limit;
pub(crate) use relay_offer_rate_limit::RelayOfferRateLimit;

mod relay_redeem_rate_limit;
pub(crate) use relay_redeem_rate_limit::RelayRedeemRateLimit;
