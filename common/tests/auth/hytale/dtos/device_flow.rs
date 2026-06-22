use common::auth::DeviceFlow;

#[test]
fn serialization_hides_device_code() {
    let flow = DeviceFlow::new(
        "secret".to_string(),
        "ABCD-1234".to_string(),
        "https://example.com/device".to_string(),
        "https://example.com/device?code=ABCD-1234".to_string(),
        600,
        5,
    );

    let json = serde_json::to_string(&flow).unwrap();

    assert!(!json.contains("secret"));
    assert!(json.contains("ABCD-1234"));
}
