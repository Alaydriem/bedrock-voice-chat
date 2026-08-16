use bvc_server_lib::services::bedrock::TransferTargetCache;

#[tokio::test]
async fn a_stored_target_is_returned_for_its_xuid() {
    let cache = TransferTargetCache::new(900);
    cache
        .set("2535428504476914", "192.168.1.100".to_string(), 19137)
        .await;

    let target = cache.get("2535428504476914").await;
    assert!(target.is_some());
    let target = target.unwrap();
    assert_eq!(target.host, "192.168.1.100");
    assert_eq!(target.port, 19137);
}

#[tokio::test]
async fn an_unknown_xuid_returns_nothing() {
    let cache = TransferTargetCache::new(900);
    let target = cache.get("0000000000000000").await;
    assert!(target.is_none());
}

#[tokio::test]
async fn a_removed_target_is_no_longer_returned() {
    let cache = TransferTargetCache::new(900);
    cache
        .set("2535428504476914", "192.168.1.100".to_string(), 19137)
        .await;
    cache.remove("2535428504476914").await;

    let target = cache.get("2535428504476914").await;
    assert!(target.is_none());
}
