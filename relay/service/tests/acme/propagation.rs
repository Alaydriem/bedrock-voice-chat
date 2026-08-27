use std::time::Duration;

use bvc_relay_service::acme::PropagationCheck;
use serde_json::json;

use crate::harness::{MockApi, MockRoute};

const FQDN: &str = "_acme-challenge.registry.example.test";
const VALUE: &str = "the-challenge-value";

fn answering(data: &str) -> serde_json::Value {
    json!({ "Status": 0, "Answer": [{ "name": FQDN, "type": 16, "data": data }] })
}

// This check is the reason an order is not wasted. A failed validation invalidates the
// whole order, and orders are the scarce thing — so the visible case must actually be
// recognised rather than always falling through to the timeout.
#[tokio::test]
async fn a_visible_record_satisfies_the_check() {
    let mock = MockApi::start(vec![MockRoute::new(
        "GET",
        "/dns-query",
        answering(&format!("\"{VALUE}\"")),
    )])
    .await;

    PropagationCheck::new_with(
        &format!("{}/dns-query", mock.base),
        Duration::from_millis(10),
        Duration::from_secs(5),
    )
    .wait_for(FQDN, VALUE)
    .await
    .expect("the record is visible");
}

// Resolvers return TXT data wrapped in quotes; the value the certificate authority
// compares is not. Without the trim every check would time out against a record that
// was in fact published — five minutes of waiting, then a failure naming DNS.
#[tokio::test]
async fn surrounding_quotes_are_not_part_of_the_value() {
    let mock = MockApi::start(vec![MockRoute::new(
        "GET",
        "/dns-query",
        answering(&format!("\"{VALUE}\"")),
    )])
    .await;

    let check = PropagationCheck::new_with(
        &format!("{}/dns-query", mock.base),
        Duration::from_millis(10),
        Duration::from_millis(500),
    );

    // The quoted form is what a resolver returns, so the unquoted value must match it
    // and the quoted one must not.
    assert!(check.wait_for(FQDN, VALUE).await.is_ok());
    assert!(
        check
            .wait_for(FQDN, &format!("\"{VALUE}\""))
            .await
            .is_err()
    );
}

// A record that never appears has to give up rather than block the start forever. A
// registry stuck here never binds its listener, and the symptom is a process that
// looks alive and answers nothing.
#[tokio::test]
async fn a_record_that_never_appears_times_out() {
    let mock = MockApi::start(vec![MockRoute::new(
        "GET",
        "/dns-query",
        answering("\"some-other-value\""),
    )])
    .await;

    let error = PropagationCheck::new_with(
        &format!("{}/dns-query", mock.base),
        Duration::from_millis(10),
        Duration::from_millis(200),
    )
    .wait_for(FQDN, VALUE)
    .await
    .expect_err("the value never appears");

    assert!(error.to_string().contains(FQDN));
}

// A resolver that answers with no records at all is indistinguishable from one that
// has not caught up, and must not be read as success.
#[tokio::test]
async fn an_empty_answer_is_not_a_match() {
    let mock = MockApi::start(vec![MockRoute::new(
        "GET",
        "/dns-query",
        json!({ "Status": 0 }),
    )])
    .await;

    assert!(
        PropagationCheck::new_with(
            &format!("{}/dns-query", mock.base),
            Duration::from_millis(10),
            Duration::from_millis(200),
        )
        .wait_for(FQDN, VALUE)
        .await
        .is_err()
    );
}
