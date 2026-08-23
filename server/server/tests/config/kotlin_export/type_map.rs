use bvc_server_lib::config::KotlinType;
use schemars::schema::Schema;

fn schema_of(json: serde_json::Value) -> Schema {
    serde_json::from_value(json).expect("valid schema")
}

fn no_defs() -> schemars::Map<String, Schema> {
    schemars::Map::new()
}

#[test]
fn maps_primitive_types() {
    let cases = [
        (serde_json::json!({"type": "string"}), "String"),
        (serde_json::json!({"type": "boolean"}), "Boolean"),
        (
            serde_json::json!({"type": "integer", "format": "uint16"}),
            "Int",
        ),
        (
            serde_json::json!({"type": "integer", "format": "uint32"}),
            "Long",
        ),
        (
            serde_json::json!({"type": "integer", "format": "uint64"}),
            "Long",
        ),
        (
            serde_json::json!({"type": "number", "format": "float"}),
            "Float",
        ),
        (
            serde_json::json!({"type": "number", "format": "double"}),
            "Double",
        ),
    ];

    for (json, expected) in cases {
        let mapped = KotlinType::of(&schema_of(json.clone()), &no_defs())
            .unwrap_or_else(|e| panic!("mapping {json} failed: {e}"));
        assert_eq!(mapped, expected, "for schema {json}");
    }
}

// u32 exceeds Int.MAX_VALUE, so a narrower mapping would silently truncate a
// port list or a capacity an operator set.
#[test]
fn maps_unsigned_32_bit_to_long() {
    let mapped = KotlinType::of(
        &schema_of(serde_json::json!({"type": "integer", "format": "uint32"})),
        &no_defs(),
    )
    .expect("uint32 maps");
    assert_eq!(mapped, "Long");
}

#[test]
fn maps_nullable_types_to_their_inner_type() {
    let mapped = KotlinType::of(
        &schema_of(serde_json::json!({"type": ["string", "null"]})),
        &no_defs(),
    )
    .expect("nullable string maps");
    assert_eq!(mapped, "String");
}

#[test]
fn maps_any_of_reference_and_null_to_the_class() {
    let mapped = KotlinType::of(
        &schema_of(serde_json::json!({
            "anyOf": [{"$ref": "#/definitions/Acme"}, {"type": "null"}]
        })),
        &no_defs(),
    )
    .expect("optional reference maps");
    assert_eq!(mapped, "Acme");
}

#[test]
fn maps_arrays_and_maps() {
    let list = KotlinType::of(
        &schema_of(serde_json::json!({"type": "array", "items": {"type": "string"}})),
        &no_defs(),
    )
    .expect("array maps");
    assert_eq!(list, "List<String>");

    let map = KotlinType::of(
        &schema_of(serde_json::json!({
            "type": "object", "additionalProperties": {"type": "boolean"}
        })),
        &no_defs(),
    )
    .expect("map maps");
    assert_eq!(map, "Map<String, Boolean>");
}

#[test]
fn rejects_a_schema_it_cannot_map() {
    let result = KotlinType::of(&schema_of(serde_json::json!({})), &no_defs());
    assert!(result.is_err(), "an untyped schema must not silently map");
}
