use common::net::HttpsProbe;
use common::structs::reachability::{AddressFamily, AnsweredVia, ReachabilityOutcome};

// Reachability and authorization are different questions. A 401 or 403 proves the
// listener is there, which is all this layer claims to measure.
#[test]
fn any_http_status_counts_as_reachable() {
    for status in [200u16, 401, 403, 404, 500] {
        match HttpsProbe::outcome_for(Some(status), 12_345) {
            ReachabilityOutcome::Answered { via, rtt_micros } => {
                assert_eq!(via, AnsweredVia::Https);
                assert_eq!(rtt_micros, 12_345);
            }
            other => panic!("status {status} gave {other:?}"),
        }
    }
}

#[test]
fn no_response_is_silent() {
    assert_eq!(
        HttpsProbe::outcome_for(None, 0),
        ReachabilityOutcome::Silent
    );
}

// A probe failure must resolve to an outcome. Propagating an error here would put
// a diagnostic on the failure path of connecting.
#[tokio::test]
async fn an_unreachable_url_is_silent_rather_than_an_error() {
    let outcome = HttpsProbe::probe("https://127.0.0.1:1/api/config", AddressFamily::Ipv4).await;

    assert_eq!(outcome, ReachabilityOutcome::Silent);
}

#[tokio::test]
async fn a_malformed_url_is_silent_rather_than_a_panic() {
    let outcome = HttpsProbe::probe("not-a-url", AddressFamily::Ipv4).await;

    assert_eq!(outcome, ReachabilityOutcome::Silent);
}
