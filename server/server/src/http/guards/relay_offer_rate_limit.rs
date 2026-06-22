use rocket_governor::{Method, Quota, RocketGovernable};

// Per-client-IP rate limit for `POST /api/relay/offer` (unauthenticated: it mints
// a code and injects it into the realm). `rocket_governor` keys the quota on the
// client IP. A distinct type from the redeem limiter so the two endpoints hold
// independent buckets (governor keys its global limiter by the implementing type).
pub struct RelayOfferRateLimit;

impl<'r> RocketGovernable<'r> for RelayOfferRateLimit {
    fn quota(_method: Method, _route_name: &str) -> Quota {
        Quota::per_minute(Self::nonzero(3u32))
    }
}
