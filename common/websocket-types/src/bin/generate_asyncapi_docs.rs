use schemars::schema_for;
use serde_json::{Value, json};
use std::fs;
use std::path::Path;
use websocket_types::*;

fn main() {
    let command_schema_value = serde_json::to_value(&schema_for!(Command)).unwrap();
    let success_schema_value = serde_json::to_value(&schema_for!(SuccessResponse)).unwrap();
    let error_schema_value = serde_json::to_value(&schema_for!(ErrorResponse)).unwrap();

    let device_type_schema = extract_def(&command_schema_value, "DeviceType");
    let pong_data_schema = extract_def(&success_schema_value, "PongData");
    let mute_data_schema = extract_def(&success_schema_value, "MuteData");
    let record_data_schema = extract_def(&success_schema_value, "RecordData");
    let state_data_schema = extract_def(&success_schema_value, "StateData");
    let response_data_schema = extract_def(&success_schema_value, "ResponseData");
    let connect_target_schema = extract_def(&success_schema_value, "ConnectTarget");
    let connect_target_kind_schema = extract_def(&success_schema_value, "ConnectTargetKind");
    let active_connection_schema = extract_def(&success_schema_value, "ActiveConnection");
    let connect_data_schema = extract_def(&success_schema_value, "ConnectData");
    let targets_data_schema = extract_def(&success_schema_value, "TargetsData");

    let command_payload = remove_defs(command_schema_value.clone());
    let success_payload = remove_defs(success_schema_value.clone());
    let error_payload = remove_defs(error_schema_value.clone());
    // The payload shape is NOT derived here. `common` is pinned to schemars 0.8 for the
    // rocket_okapi stack while this crate is on 1.0, so the two `JsonSchema` traits are
    // incompatible and the snapshot type cannot be reflected from here. Rather than hand-copy the
    // shape — which would drift from the Rust definition with nothing to catch it — the channel
    // documents its envelope and points at the generated TypeScript binding, which is produced
    // from that single definition and is the contract a consumer should read.
    let metrics_payload = json!({
        "type": "object",
        "required": ["type", "data"],
        "properties": {
            "type": {
                "type": "string",
                "const": "metrics",
                "description": "Discriminant. Present so a consumer can tell this from a state frame."
            },
            "data": {
                "type": "object",
                "description": "LinkDiagnosticsSnapshot. Field-level contract: client/src/js/bindings/LinkDiagnosticsSnapshot.ts, generated from the Rust definition in common/src/structs/metrics/."
            }
        }
    });

    let spec = json!({
        "asyncapi": "3.0.0",
        "info": {
            "title": "Bedrock Voice Chat WebSocket API",
            "version": "1.0.0",
            "description": "A local WebSocket API for controlling Bedrock Voice Chat client via Stream Deck and other integrations"
        },
        "servers": {
            "local": {
                "host": "127.0.0.1:9595",
                "protocol": "ws",
                "description": "Local websocket server running from Bedrock Voice Chat client"
            }
        },
        "channels": {
            "root": {
                "address": "/",
                "messages": {
                    "command": {
                        "$ref": "#/components/messages/Command"
                    },
                    "success": {
                        "$ref": "#/components/messages/SuccessResponse"
                    },
                    "error": {
                        "$ref": "#/components/messages/ErrorResponse"
                    }
                }
            },
            "metrics": {
                "address": "/metrics",
                "description": "Push-only link diagnostics, one frame per second while a connection is live. Authenticate with the configured key as a `key` query parameter; the upgrade is refused outright when it is missing or wrong. Inbound frames other than close and ping are ignored.",
                "messages": {
                    "metrics": {
                        "$ref": "#/components/messages/MetricsPush"
                    }
                }
            }
        },
        "components": {
            "messages": {
                "Command": {
                    "name": "Command",
                    "title": "WebSocket Command",
                    "summary": "Commands that can be sent to the WebSocket server",
                    "description": "Tagged union of all available commands (ping, mute, record, state, ptt, targets, connect, disconnect)",
                    "contentType": "application/json",
                    "payload": command_payload
                },
                "SuccessResponse": {
                    "name": "SuccessResponse",
                    "title": "Success Response",
                    "summary": "Response sent when command succeeds",
                    "description": "Contains success flag and command-specific data",
                    "contentType": "application/json",
                    "payload": success_payload
                },
                "MetricsPush": {
                    "name": "MetricsPush",
                    "title": "Link Diagnostics Push",
                    "summary": "Live link, device and per-speaker diagnostics",
                    "description": "Tagged envelope carrying a full diagnostics snapshot. Tagged rather than riding on ResponseData, which is an untagged union a consumer could not discriminate.",
                    "contentType": "application/json",
                    "payload": metrics_payload
                },
                "ErrorResponse": {
                    "name": "ErrorResponse",
                    "title": "Error Response",
                    "summary": "Response sent when command fails",
                    "description": "Contains success flag (false) and error message",
                    "contentType": "application/json",
                    "payload": error_payload
                }
            },
            "schemas": {
                "DeviceType": device_type_schema,
                "PongData": pong_data_schema,
                "MuteData": mute_data_schema,
                "RecordData": record_data_schema,
                "StateData": state_data_schema,
                "ResponseData": response_data_schema,
                "ConnectTarget": connect_target_schema,
                "ConnectTargetKind": connect_target_kind_schema,
                "ActiveConnection": active_connection_schema,
                "ConnectData": connect_data_schema,
                "TargetsData": targets_data_schema
            }
        }
    });

    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let output_path = Path::new(manifest_dir).join("../../docs/websocket-api.yaml");
    fs::create_dir_all(output_path.parent().unwrap()).expect("Failed to create docs directory");

    let yaml = serde_yaml::to_string(&spec).expect("Failed to serialize AsyncAPI spec to YAML");

    fs::write(&output_path, yaml).expect("Failed to write AsyncAPI spec");

    println!(
        "Generated AsyncAPI spec at: {:?}",
        output_path.canonicalize().unwrap()
    );
}

fn extract_def(schema_value: &Value, def_name: &str) -> Value {
    let mut def = schema_value
        .get("$defs")
        .and_then(|defs| defs.get(def_name))
        .cloned()
        .unwrap_or(json!({}));

    update_refs(&mut def);

    def
}

fn remove_defs(mut schema_value: Value) -> Value {
    if let Some(obj) = schema_value.as_object_mut() {
        obj.remove("$defs");
    }

    update_refs(&mut schema_value);

    schema_value
}

fn update_refs(value: &mut Value) {
    match value {
        Value::Object(map) => {
            if let Some(Value::String(ref_str)) = map.get("$ref") {
                if ref_str.starts_with("#/$defs/") {
                    let def_name = ref_str.strip_prefix("#/$defs/").unwrap();
                    map.insert(
                        "$ref".to_string(),
                        Value::String(format!("#/components/schemas/{}", def_name)),
                    );
                }
            }
            for val in map.values_mut() {
                update_refs(val);
            }
        }
        Value::Array(arr) => {
            for val in arr {
                update_refs(val);
            }
        }
        _ => {}
    }
}
