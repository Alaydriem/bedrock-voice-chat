use rocket_governor::{Method, Quota, RocketGovernable};

// Per-client-IP rate limit for `POST /api/relay/peer-redeem` (unauthenticated code
// redemption). Bounds brute-force redemption attempts. A distinct type from the
// offer limiter so the two endpoints hold independent buckets (governor keys its
// global limiter by the implementing type).
pub struct RelayRedeemRateLimit;

impl<'r> RocketGovernable<'r> for RelayRedeemRateLimit {
    fn quota(_method: Method, _route_name: &str) -> Quota {
        Quota::per_minute(Self::nonzero(super::RELAY_CODE_RATE_PER_MINUTE))
    }
}
