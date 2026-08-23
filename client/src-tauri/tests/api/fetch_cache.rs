use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use bvc_client_lib::FetchCache;

fn cache() -> FetchCache<u32> {
    FetchCache::new(Duration::from_secs(30), 16)
}

// The whole point: four consumers in one launch, one round trip.
#[tokio::test]
async fn repeat_calls_inside_the_ttl_fetch_once() {
    let cache = cache();
    let calls = Arc::new(AtomicUsize::new(0));

    for _ in 0..4 {
        let calls = calls.clone();
        let got = cache
            .get_or_fetch("https://a.example", || async move {
                calls.fetch_add(1, Ordering::SeqCst);
                Ok(443)
            })
            .await
            .expect("cached fetch must succeed");
        assert_eq!(got, 443);
    }

    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

// Two servers are two documents. A cache keyed too loosely would answer one with the other.
#[tokio::test]
async fn each_key_is_cached_separately() {
    let cache = cache();

    let first = cache
        .get_or_fetch("https://a.example", || async { Ok(443) })
        .await
        .expect("first fetch must succeed");
    let second = cache
        .get_or_fetch("https://b.example", || async { Ok(8443) })
        .await
        .expect("second fetch must succeed");

    assert_eq!(first, 443);
    assert_eq!(second, 8443);
}

// A connect that failed because the port moved has to be able to re-ask. Without this the
// cache turns a recoverable failure into one that stays broken for the whole TTL.
#[tokio::test]
async fn invalidate_forces_the_next_call_to_refetch() {
    let cache = cache();
    let calls = Arc::new(AtomicUsize::new(0));

    for _ in 0..2 {
        let calls = calls.clone();
        _ = cache
            .get_or_fetch("https://a.example", || async move {
                calls.fetch_add(1, Ordering::SeqCst);
                Ok(443)
            })
            .await;
        cache.invalidate("https://a.example").await;
    }

    assert_eq!(calls.load(Ordering::SeqCst), 2);
}

// A cached error would hand the same failure to every later caller in the window, and the
// retry that would have succeeded never happens.
#[tokio::test]
async fn a_failed_fetch_is_not_cached() {
    let cache = cache();

    let failed = cache
        .get_or_fetch("https://a.example", || async {
            Err("server unreachable".to_string())
        })
        .await;
    assert!(failed.is_err());

    let recovered = cache
        .get_or_fetch("https://a.example", || async { Ok(443) })
        .await
        .expect("a failed fetch must not poison the entry");
    assert_eq!(recovered, 443);
}

// An entry past its TTL is a question worth re-asking: this document decides which port to
// dial, and the server can move it.
#[tokio::test]
async fn an_expired_entry_is_refetched() {
    let cache = FetchCache::<u32>::new(Duration::from_millis(50), 16);
    let calls = Arc::new(AtomicUsize::new(0));

    for _ in 0..2 {
        let calls = calls.clone();
        _ = cache
            .get_or_fetch("https://a.example", || async move {
                calls.fetch_add(1, Ordering::SeqCst);
                Ok(443)
            })
            .await;
        tokio::time::sleep(Duration::from_millis(80)).await;
    }

    assert_eq!(calls.load(Ordering::SeqCst), 2);
}
