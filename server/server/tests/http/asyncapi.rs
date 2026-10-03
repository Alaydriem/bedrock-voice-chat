//! AsyncApiSpec::generate
//!
//! Contract:
//! - every `$ref` in the document resolves inside the document. A dangling one renders as an
//!   empty payload and nothing else reports it.
//! - every operation names a channel and a security scheme that exist.

use bvc_server_lib::http::asyncapi::AsyncApiSpec;
use serde_json::Value;

fn collect_refs(value: &Value, refs: &mut Vec<String>) {
    match value {
        Value::Object(map) => {
            if let Some(Value::String(target)) = map.get("$ref") {
                refs.push(target.clone());
            }
            map.values().for_each(|inner| collect_refs(inner, refs));
        }
        Value::Array(items) => items.iter().for_each(|inner| collect_refs(inner, refs)),
        _ => {}
    }
}

fn resolve<'a>(spec: &'a Value, target: &str) -> Option<&'a Value> {
    let pointer = target.strip_prefix('#')?;
    spec.pointer(pointer)
}

#[test]
fn every_ref_resolves() {
    let spec = AsyncApiSpec::generate();
    let mut refs = Vec::new();
    collect_refs(&spec, &mut refs);

    assert!(!refs.is_empty());
    let dangling: Vec<&String> = refs
        .iter()
        .filter(|target| resolve(&spec, target).is_none())
        .collect();
    assert!(dangling.is_empty(), "dangling $refs: {dangling:?}");
}

#[test]
fn payloads_are_reflected_from_the_wire_types() {
    let spec = AsyncApiSpec::generate();
    let schemas = &spec["components"]["schemas"];

    let snapshot = &schemas["PositionSnapshot"]["properties"];
    assert!(snapshot.get("seq").is_some());
    assert!(snapshot.get("positions").is_some());
    assert!(schemas["RelativePosition"]["properties"].get("bearing_deg").is_some());
    assert!(schemas.get("ChatFrame").is_some());
}

#[test]
fn channel_addresses_sit_under_the_api_prefix() {
    let spec = AsyncApiSpec::generate();

    assert_eq!(spec["channels"]["positions"]["address"], "/api/websocket/positions");
    assert_eq!(spec["channels"]["chat"]["address"], "/api/websocket/chat");
}
