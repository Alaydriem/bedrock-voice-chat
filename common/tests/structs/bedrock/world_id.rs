use common::structs::bedrock::BedrockWorldId;

#[test]
fn derive_is_stable() {
    let actual = BedrockWorldId::derive(123, "level-id", "My World");
    assert_eq!(
        actual,
        "cba824f1284f220f787dc7f56a42fa03fb40f885c43412a2b7287f85238c882d",
    );
}

#[test]
fn derive_is_deterministic() {
    assert_eq!(
        BedrockWorldId::derive(42, "abc", "Hello"),
        BedrockWorldId::derive(42, "abc", "Hello"),
    );
}

#[test]
fn derive_diverges_on_any_field_change() {
    let a = BedrockWorldId::derive(1, "abc", "World");
    let b = BedrockWorldId::derive(2, "abc", "World");
    let c = BedrockWorldId::derive(1, "def", "World");
    let d = BedrockWorldId::derive(1, "abc", "Other");
    assert_ne!(a, b);
    assert_ne!(a, c);
    assert_ne!(a, d);
}
