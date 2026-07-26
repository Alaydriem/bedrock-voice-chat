use bvc_server_lib::services::acme::AcmeDnsProvider;

use super::stub::StubServer;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn publish_posts_update_with_credentials() {
    let stub = StubServer::launch().await;
    let provider = AcmeDnsProvider::new(
        &stub.base_url,
        "the-user",
        "the-password",
        "d420c923-bbd7-4056-ab64-c3ca54c9b3cf",
    );

    provider
        .publish_txt("voice.example.com", "txt-value-43-chars")
        .await
        .unwrap();

    let requests = stub.state.requests.lock().unwrap().clone();
    assert_eq!(requests.len(), 1);
    let req = &requests[0];
    assert_eq!(req.method, "POST");
    assert_eq!(req.path, "/update");
    assert_eq!(req.api_user.as_deref(), Some("the-user"));
    assert_eq!(req.api_key.as_deref(), Some("the-password"));
    let body = req.body.as_ref().unwrap();
    assert_eq!(body["subdomain"], "d420c923-bbd7-4056-ab64-c3ca54c9b3cf");
    assert_eq!(body["txt"], "txt-value-43-chars");
}
