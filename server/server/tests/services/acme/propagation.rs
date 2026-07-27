use std::time::Duration;

use bvc_server_lib::services::acme::PropagationChecker;
use serde_json::json;

use super::stub::StubServer;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn returns_once_txt_value_is_visible() {
    let stub = StubServer::launch().await;
    stub.state.responses.lock().unwrap().push((
        "/dns-query".to_string(),
        json!({"Status": 0, "Answer": [
            {"name": "_acme-challenge.voice.example.com", "type": 16, "data": "\"expected-token\""}
        ]}),
    ));

    let checker = PropagationChecker::new_with(
        &format!("{}/dns-query", stub.base_url),
        Duration::from_millis(50),
        Duration::from_secs(5),
    );
    checker
        .wait_for_txt("_acme-challenge.voice.example.com", "expected-token")
        .await
        .unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn times_out_when_value_never_appears() {
    let stub = StubServer::launch().await;
    stub.state.responses.lock().unwrap().push((
        "/dns-query".to_string(),
        json!({"Status": 0, "Answer": []}),
    ));

    let checker = PropagationChecker::new_with(
        &format!("{}/dns-query", stub.base_url),
        Duration::from_millis(50),
        Duration::from_millis(300),
    );
    let err = checker
        .wait_for_txt("_acme-challenge.voice.example.com", "expected-token")
        .await
        .unwrap_err();
    assert!(format!("{err}").to_lowercase().contains("propagat"));
}
