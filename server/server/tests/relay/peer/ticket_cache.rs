use std::time::Duration;

use bvc_server_lib::relay::PeerTicketCache;

const LONG: Duration = Duration::from_secs(300);

// Concurrent readers of a cold cache must start one mint between them, not one each. Each
// mint is a registry round trip, and a burst of `/api/config` requests used to start one
// per request.
#[test]
fn only_one_reader_claims_a_missing_link() {
    let cache = PeerTicketCache::new(LONG);

    assert!(cache.claim_refresh());
    assert!(!cache.claim_refresh());
}

#[test]
fn a_fresh_link_needs_no_refresh() {
    let cache = PeerTicketCache::new(LONG);
    assert!(cache.claim_refresh());
    cache.store("link".to_string());

    assert!(!cache.claim_refresh());
    assert_eq!(cache.get().as_deref(), Some("link"));
}

// A stale link is served while its replacement is minted, rather than withheld until the
// registry answers.
#[test]
fn a_stale_link_is_still_served_while_one_reader_refreshes_it() {
    let cache = PeerTicketCache::new(Duration::ZERO);
    assert!(cache.claim_refresh());
    cache.store("old".to_string());

    assert!(cache.claim_refresh());
    assert!(!cache.claim_refresh());
    assert_eq!(cache.get().as_deref(), Some("old"));
}

// A mint that produced nothing must not leave the cache believing one is still running,
// or no reader would ever try again.
#[test]
fn a_released_refresh_can_be_claimed_again() {
    let cache = PeerTicketCache::new(LONG);
    assert!(cache.claim_refresh());
    cache.release();

    assert!(cache.claim_refresh());
    assert_eq!(cache.get(), None);
}
