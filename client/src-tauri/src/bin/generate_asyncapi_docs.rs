// This binary is only useful on desktop, but we need a main() for all targets
#[cfg(not(desktop))]
fn main() {
    eprintln!("AsyncAPI docs generation is only supported on desktop platforms");
}

#[cfg(desktop)]
use schemars::schema_for;
#[cfg(desktop)]
use bvc_client_lib::websocket::structs::*;
#[cfg(desktop)]
use serde_json::{json, Value};
#[cfg(desktop)]
use std::fs;
#[cfg(desktop)]
use std::path::Path;

#[cfg(desktop)]
fn main() {
    // Generate JSON schemas for our types
    let command_schema_value = serde_json::to_value(&schema_for!(Command)).unwrap();
    let success_schema_value = serde_json::to_value(&schema_for!(SuccessResponse)).unwrap();
    let error_schema_value = serde_json::to_value(&schema_for!(ErrorResponse)).unwrap();

    // Extract nested definitions for AsyncAPI components
    let device_type_schema = extract_def(&command_schema_value, "DeviceType");
    let pong_data_schema = extract_def(&success_schema_value, "PongData");
    let mute_data_schema = extract_def(&success_schema_value, "MuteData");
    let record_data_schema = extract_def(&success_schema_value, "RecordData");
    let state_data_schema = extract_def(&success_schema_value, "StateData");
    let response_data_schema = extract_def(&success_schema_value, "ResponseData");

    // Remove $defs from payloads and update refs
    let command_payload = remove_defs(command_schema_value.clone());
    let success_payload = remove_defs(success_schema_value.clone());
    let error_payload = remove_defs(error_schema_value.clone());

    // Build AsyncAPI spec as a JSON value
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
            }
        },
        "components": {
            "messages": {
                "Command": {
                    "name": "Command",
                    "title": "WebSocket Command",
                    "summary": "Commands that can be sent to the WebSocket server",
                    "description": "Tagged union of all available commands (ping, mute, record)",
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
                "ResponseData": response_data_schema
            }
        }
    });

    // Write AsyncAPI spec to file
    let output_path = Path::new("../../docs/websocket-api.yaml");
    fs::create_dir_all(output_path.parent().unwrap())
        .expect("Failed to create docs directory");

    let yaml = serde_yaml::to_string(&spec)
        .expect("Failed to serialize AsyncAPI spec to YAML");

    fs::write(output_path, yaml)
        .expect("Failed to write AsyncAPI spec");

    println!("✓ Generated AsyncAPI spec at: {:?}", output_path.canonicalize().unwrap());
}

#[cfg(desktop)]
fn extract_def(schema_value: &Value, def_name: &str) -> Value {
    let mut def = schema_value.get("$defs")
        .and_then(|defs| defs.get(def_name))
        .cloned()
        .unwrap_or(json!({}));

    // Update refs in the extracted definition too
    update_refs(&mut def);

    def
}

#[cfg(desktop)]
fn remove_defs(mut schema_value: Value) -> Value {
    // Remove $defs from schema and update references to point to components
    if let Some(obj) = schema_value.as_object_mut() {
        obj.remove("$defs");
    }

    // Update all $ref paths from #/$defs/X to #/components/schemas/X
    update_refs(&mut schema_value);

    schema_value
}

#[cfg(desktop)]
fn update_refs(value: &mut Value) {
    match value {
        Value::Object(map) => {
            if let Some(Value::String(ref_str)) = map.get("$ref") {
                if ref_str.starts_with("#/$defs/") {
                    let def_name = ref_str.strip_prefix("#/$defs/").unwrap();
                    map.insert("$ref".to_string(), Value::String(format!("#/components/schemas/{}", def_name)));
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
