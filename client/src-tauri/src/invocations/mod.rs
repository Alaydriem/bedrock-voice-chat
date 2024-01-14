pub(crate) mod credentials;
pub(crate) mod login;
pub(crate) mod stream;
pub(crate) mod network;

use reqwest::Client;
use std::time::Duration;
use moka::future::Cache;
use std::sync::Arc;
use std::collections::HashMap;

/// Returns an Reqwest Client configured to talk to BVC API
pub(crate) fn get_reqwest_client() -> Client {
    let mut builder = reqwest::Client
        ::builder()
        .use_rustls_tls()
        .timeout(Duration::new(5, 0))
        .danger_accept_invalid_certs(false);

    // In debug builds, allow invalid or bad certificates for testing
    #[cfg(debug_assertions)]
    {
        builder = builder.danger_accept_invalid_certs(true);
    }

    return builder.build().unwrap();
}

/// Determines if a thread should self terminate or not by looking up the job ID inside of the named cache
/// Lookup the jobs inside of cache state
/// If the ID of this job no longer exists in it (or other states have changed) then self terminate
/// We should also terminate if the cache fails
/// This lookup takes 80-150.6µs, which shouldn't interfere with any audio playback buffering checks
pub(crate) async fn should_self_terminate(
    id: &str,
    cache: &Arc<Cache<String, String>>,
    cache_key: &str
) -> bool {
    match cache.get(cache_key).await {
        Some(result) => {
            let jobs: HashMap<String, i8> = serde_json::from_str(&result).unwrap();
            match jobs.get(id) {
                Some(_) => {
                    return false;
                }
                None => {
                    return true;
                }
            }
        }
        None => {
            return true;
        }
    }
}
