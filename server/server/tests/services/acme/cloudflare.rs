use bvc_server_lib::services::acme::CloudflareProvider;
use serde_json::json;

use super::stub::StubServer;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn publish_finds_zone_and_creates_txt_record() {
    let stub = StubServer::launch().await;
    // Order matters: the dns_records prefix must be checked before /zones.
    stub.state.responses.lock().unwrap().push((
        "/zones/zone-123/dns_records".to_string(),
        json!({"success": true, "result": {"id": "rec-1"}}),
    ));
    stub.state.responses.lock().unwrap().push((
        "/zones".to_string(),
        json!({"success": true, "result": [{"id": "zone-123", "name": "example.com"}]}),
    ));

    let provider = CloudflareProvider::new_with_base("cf-token", &stub.base_url);
    provider
        .publish_txt("voice.example.com", "txt-value")
        .await
        .unwrap();

    let requests = stub.state.requests.lock().unwrap().clone();
    let create = requests
        .iter()
        .find(|r| r.method == "POST")
        .expect("a record-create POST must happen");
    assert!(create.path.starts_with("/zones/zone-123/dns_records"));
    assert_eq!(create.authorization.as_deref(), Some("Bearer cf-token"));
    let body = create.body.as_ref().unwrap();
    assert_eq!(body["type"], "TXT");
    assert_eq!(body["name"], "_acme-challenge.voice.example.com");
    assert_eq!(body["content"], "txt-value");

    let lookups: Vec<_> = requests.iter().filter(|r| r.method == "GET").collect();
    assert!(!lookups.is_empty(), "zone lookup must happen before create");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cleanup_deletes_matching_records() {
    let stub = StubServer::launch().await;
    stub.state.responses.lock().unwrap().push((
        "/zones/zone-123/dns_records".to_string(),
        json!({"success": true, "result": [{"id": "rec-9"}]}),
    ));
    stub.state.responses.lock().unwrap().push((
        "/zones".to_string(),
        json!({"success": true, "result": [{"id": "zone-123", "name": "example.com"}]}),
    ));

    let provider = CloudflareProvider::new_with_base("cf-token", &stub.base_url);
    provider.cleanup_txt("voice.example.com").await.unwrap();

    let requests = stub.state.requests.lock().unwrap().clone();
    let delete = requests
        .iter()
        .find(|r| r.method == "DELETE")
        .expect("a DELETE must happen");
    assert!(delete.path.ends_with("/dns_records/rec-9"));
}
