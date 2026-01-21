use common::structs::config::{HytaleDeviceFlowStartResponse, HytaleDeviceFlowStatusResponse};
use serde::{Deserialize, Serialize};

use base64::{Engine as _, engine::general_purpose};

use crate::auth::ncryptf;

/// Client-side JsonMessage for deserializing server responses
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JsonMessage<T> {
    pub status: u16,
    pub data: Option<T>,
    pub message: Option<String>,
}

const HYTALE_START_DEVICE_FLOW_ENDPOINT: &str = "api/auth/hytale/start-device-flow";

/// Start a Hytale device flow authentication
pub async fn start_hytale_device_flow(
    server: String,
) -> Result<HytaleDeviceFlowStartResponse, bool> {
    // Get ncryptf encryption key
    let ek = match ncryptf::get_ek(server.clone()).await {
        Ok(ek) => ek,
        Err(e) => {
            log::error!("Failed to get encryption key: {:?}", e);
            return Err(false);
        }
    };

    let kp = common::ncryptflib::Keypair::new();

    let mut headers = tauri_plugin_http::reqwest::header::HeaderMap::new();
    headers.insert(
        "Content-Type",
        tauri_plugin_http::reqwest::header::HeaderValue::from_str("application/json").unwrap(),
    );
    headers.insert(
        "Accept",
        tauri_plugin_http::reqwest::header::HeaderValue::from_str("application/vnd.ncryptf+json")
            .unwrap(),
    );
    headers.insert(
        "X-HashId",
        tauri_plugin_http::reqwest::header::HeaderValue::from_str(&ek.hash_id).unwrap(),
    );
    headers.insert(
        "X-PubKey",
        tauri_plugin_http::reqwest::header::HeaderValue::from_str(
            &general_purpose::STANDARD.encode(kp.get_public_key()),
        )
        .unwrap(),
    );

    let endpoint = format!("{}/{}", &server, HYTALE_START_DEVICE_FLOW_ENDPOINT);
    let client = ncryptf::get_reqwest_client();

    match client.post(endpoint).headers(headers).send().await {
        Ok(response) => {
            if response.status() == tauri_plugin_http::reqwest::StatusCode::OK {
                match response.bytes().await {
                    Ok(bytes) => {
                        let bbody = match general_purpose::STANDARD.decode(bytes.clone()) {
                            Ok(decoded) => decoded,
                            Err(e) => {
                                log::error!("Failed to decode base64 response: {:?}", e);
                                return Err(false);
                            }
                        };

                        let r = match common::ncryptflib::Response::from(kp.get_secret_key()) {
                            Ok(r) => r,
                            Err(e) => {
                                log::error!("Failed to create ncryptf response: {:?}", e);
                                return Err(false);
                            }
                        };

                        match r.decrypt(bbody, None, None) {
                            Ok(json) => {
                                match serde_json::from_str::<
                                    JsonMessage<HytaleDeviceFlowStartResponse>,
                                >(&json)
                                {
                                    Ok(response) => match response.data {
                                        Some(data) => Ok(data),
                                        None => {
                                            log::error!("No data in response");
                                            Err(false)
                                        }
                                    },
                                    Err(e) => {
                                        log::error!("Failed to parse response: {:?}", e);
                                        Err(false)
                                    }
                                }
                            }
                            Err(e) => {
                                log::error!("Failed to decrypt response: {:?}", e);
                                Err(false)
                            }
                        }
                    }
                    Err(e) => {
                        log::error!("Failed to read response bytes: {:?}", e);
                        Err(false)
                    }
                }
            } else {
                log::error!("Request failed with status: {}", response.status());
                Err(false)
            }
        }
        Err(e) => {
            log::error!("Request failed: {:?}", e);
            Err(false)
        }
    }
}

/// Poll the status of a Hytale device flow
pub async fn poll_hytale_status(
    server: String,
    session_id: String,
) -> Result<HytaleDeviceFlowStatusResponse, bool> {
    // Get ncryptf encryption key
    let ek = match ncryptf::get_ek(server.clone()).await {
        Ok(ek) => ek,
        Err(e) => {
            log::error!("Failed to get encryption key: {:?}", e);
            return Err(false);
        }
    };

    let kp = common::ncryptflib::Keypair::new();

    let mut headers = tauri_plugin_http::reqwest::header::HeaderMap::new();
    headers.insert(
        "Accept",
        tauri_plugin_http::reqwest::header::HeaderValue::from_str("application/vnd.ncryptf+json")
            .unwrap(),
    );
    headers.insert(
        "X-HashId",
        tauri_plugin_http::reqwest::header::HeaderValue::from_str(&ek.hash_id).unwrap(),
    );
    headers.insert(
        "X-PubKey",
        tauri_plugin_http::reqwest::header::HeaderValue::from_str(
            &general_purpose::STANDARD.encode(kp.get_public_key()),
        )
        .unwrap(),
    );

    let endpoint = format!("{}/api/auth/hytale/status/{}", &server, session_id);
    let client = ncryptf::get_reqwest_client();

    match client.get(endpoint).headers(headers).send().await {
        Ok(response) => {
            let status = response.status();
            if status == tauri_plugin_http::reqwest::StatusCode::OK {
                match response.bytes().await {
                    Ok(bytes) => {
                        let bbody = match general_purpose::STANDARD.decode(bytes.clone()) {
                            Ok(decoded) => decoded,
                            Err(e) => {
                                log::error!("Failed to decode base64 response: {:?}", e);
                                return Err(false);
                            }
                        };

                        let r = match common::ncryptflib::Response::from(kp.get_secret_key()) {
                            Ok(r) => r,
                            Err(e) => {
                                log::error!("Failed to create ncryptf response: {:?}", e);
                                return Err(false);
                            }
                        };

                        match r.decrypt(bbody, None, None) {
                            Ok(json) => {
                                match serde_json::from_str::<
                                    JsonMessage<HytaleDeviceFlowStatusResponse>,
                                >(&json)
                                {
                                    Ok(response) => match response.data {
                                        Some(data) => Ok(data),
                                        None => {
                                            log::error!("No data in response");
                                            Err(false)
                                        }
                                    },
                                    Err(e) => {
                                        log::error!("Failed to parse response: {:?}", e);
                                        Err(false)
                                    }
                                }
                            }
                            Err(e) => {
                                log::error!("Failed to decrypt response: {:?}", e);
                                Err(false)
                            }
                        }
                    }
                    Err(e) => {
                        log::error!("Failed to read response bytes: {:?}", e);
                        Err(false)
                    }
                }
            } else if status == tauri_plugin_http::reqwest::StatusCode::NOT_FOUND {
                log::error!("Session not found");
                Err(false)
            } else if status == tauri_plugin_http::reqwest::StatusCode::FORBIDDEN {
                log::error!("Player not found or banished");
                Err(false)
            } else {
                log::error!("Request failed with status: {}", status);
                Err(false)
            }
        }
        Err(e) => {
            log::error!("Request failed: {:?}", e);
            Err(false)
        }
    }
}
