use common::{
    ncryptflib::rocket::{ base64, ExportableEncryptionKeyData },
    structs::{ config::{ ApiConfig, LoginRequest, LoginResponse }, ncryptf_json::JsonMessage },
};

use reqwest::{ header::{ HeaderMap, HeaderValue }, Client, StatusCode };
use common::structs::config::MicrosoftAuthCodeAndUrlResponse;

use crate::invocations::get_reqwest_client;

use super::{ credentials::del_credential, stream::stop_stream, network::stop_network_stream };
const CONFIG_ENDPOINT: &'static str = "/api/config";
const AUTH_ENDPOINT: &'static str = "/api/auth";
const NCRYPTF_EK_ENDPOINT: &'static str = "/ncryptf/ek";

pub(crate) fn get_base_endpoint(server: String, endpoint: String) -> (Client, String) {
    let client = get_reqwest_client();
    let endpoint = format!("https://{}{}", server.replace("https://", ""), endpoint);

    (client, endpoint)
}

pub(crate) async fn get_ncryptf_ek(
    server: String
) -> Result<ExportableEncryptionKeyData, anyhow::Error> {
    let (client, endpoint) = get_base_endpoint(server, NCRYPTF_EK_ENDPOINT.to_string());

    let ek: ExportableEncryptionKeyData = client
        .get(endpoint)
        .send().await?
        .json::<ExportableEncryptionKeyData>().await?;

    Ok(ek)
}

/// Checks the API and ensures that we can connect to it.
#[tauri::command(async)]
pub(crate) async fn check_api_status(server: String) -> Result<ApiConfig, bool> {
    let (client, endpoint) = get_base_endpoint(server, CONFIG_ENDPOINT.to_string());

    match client.get(endpoint).send().await {
        Ok(response) =>
            match response.status() {
                StatusCode::OK =>
                    match response.json::<common::structs::config::ApiConfig>().await {
                        Ok(data) => {
                            tracing::info!("Client connected to Server!");
                            // At a later point, we might want to check certain elements
                            return Ok(data);
                        }
                        Err(_) => {
                            return Err(false);
                        }
                    }
                _ => {
                    return Err(false);
                }
            }
        Err(_) => {
            return Err(false);
        }
    }
}

#[tauri::command(async)]
pub(crate) async fn microsoft_auth(cid: String) -> Result<MicrosoftAuthCodeAndUrlResponse, bool> {
    tracing::info!("Starting Authentication Step 1 with Client ID: {}", cid);
    match common::auth::xbl::client_authenticate_step_1(cid).await {
        Ok((url, state)) => Ok(MicrosoftAuthCodeAndUrlResponse { url, state }),
        Err(e) => {
            tracing::error!("{}", e.to_string());
            return Err(false);
        }
    }
}

#[tauri::command(async)]
pub(crate) async fn microsoft_auth_listener(state: String) -> Result<String, bool> {
    match common::auth::xbl::client_authenticate_step_2(state).await {
        Ok(code) => Ok(code),
        Err(e) => {
            tracing::error!("{}", e.to_string());
            Err(false)
        }
    }
}

#[tauri::command(async)]
pub(crate) async fn microsoft_auth_login(
    server: String,
    code: String
) -> Result<LoginResponse, bool> {
    let (client, endpoint) = get_base_endpoint(server.clone(), AUTH_ENDPOINT.to_string());
    let payload = LoginRequest { code };

    // We're going to setup an ncryptf client
    let ek = match get_ncryptf_ek(server).await {
        Ok(ek) => ek,
        Err(_) => {
            return Err(false);
        }
    };

    let kp = common::ncryptflib::Keypair::new();

    let mut headers = HeaderMap::new();
    headers.insert("Content-Type", HeaderValue::from_str("application/json").unwrap());
    headers.insert("Accept", HeaderValue::from_str("application/vnd.ncryptf+json").unwrap());
    headers.insert("X-HashId", HeaderValue::from_str(&ek.hash_id).unwrap());
    headers.insert(
        "X-PubKey",
        HeaderValue::from_str(&base64::encode(kp.get_public_key())).unwrap()
    );

    match client.post(endpoint).headers(headers).json(&payload).send().await {
        Ok(response) => {
            match response.status() {
                StatusCode::OK => {
                    match response.bytes().await {
                        Ok(bytes) => {
                            let bbody = base64::decode(bytes.clone()).unwrap();
                            let r = common::ncryptflib::Response
                                ::from(kp.get_secret_key())
                                .unwrap();

                            match r.decrypt(bbody, None, None) {
                                Ok(json) =>
                                    match serde_json::from_str::<JsonMessage<LoginResponse>>(&json) {
                                        Ok(response) => {
                                            match response.data {
                                                Some(mut data) => {
                                                    let kp = data.clone().keypair;
                                                    let sk = data.clone().signature;

                                                    // Store data in CredentialVault
                                                    crate::invocations::credentials::set_credential(
                                                        "gamertag".to_string(),
                                                        data.clone().gamertag
                                                    ).await;

                                                    crate::invocations::credentials::set_credential(
                                                        "gamerpic".to_string(),
                                                        data.clone().gamerpic
                                                    ).await;

                                                    crate::invocations::credentials::set_credential(
                                                        "key_pk".to_string(),
                                                        base64::encode(kp.pk)
                                                    ).await;

                                                    crate::invocations::credentials::set_credential(
                                                        "key_sk".to_string(),
                                                        base64::encode(kp.sk)
                                                    ).await;

                                                    crate::invocations::credentials::set_credential(
                                                        "sig_pk".to_string(),
                                                        base64::encode(sk.pk)
                                                    ).await;

                                                    crate::invocations::credentials::set_credential(
                                                        "sig_sk".to_string(),
                                                        base64::encode(sk.sk)
                                                    ).await;

                                                    crate::invocations::credentials::set_credential(
                                                        "certificate".to_string(),
                                                        data.clone().certificate
                                                    ).await;

                                                    crate::invocations::credentials::set_credential(
                                                        "key".to_string(),
                                                        data.clone().certificate_key
                                                    ).await;

                                                    crate::invocations::credentials::set_credential(
                                                        "ca".to_string(),
                                                        data.clone().certificate_ca
                                                    ).await;

                                                    crate::invocations::credentials::set_credential(
                                                        "quic_connect_string".to_string(),
                                                        data.clone().quic_connect_string
                                                    ).await;

                                                    // Only return the gamertag and gamerpic, the rest we don't want to expose to the frontend
                                                    data.keypair =
                                                        common::structs::config::Keypair {
                                                            pk: Vec::<u8>::new(),
                                                            sk: Vec::<u8>::new(),
                                                        };
                                                    data.signature =
                                                        common::structs::config::Keypair {
                                                            pk: Vec::<u8>::new(),
                                                            sk: Vec::<u8>::new(),
                                                        };
                                                    Ok(data)
                                                }
                                                None => Err(false),
                                            }
                                        }
                                        Err(e) => {
                                            tracing::error!("{:?}", e.to_string());
                                            Err(false)
                                        }
                                    }
                                Err(e) => {
                                    tracing::error!("{}", e.to_string());
                                    return Err(false);
                                }
                            }
                        }
                        Err(e) => {
                            tracing::error!("{}", e.to_string());
                            return Err(false);
                        }
                    }
                }
                _ => Err(false),
            }
        }
        Err(e) => {
            tracing::error!("{}", e.to_string());
            Err(false)
        }
    }
}

#[tauri::command(async)]
pub async fn logout() {
    // Terminate both the input and output stream
    stop_stream(common::structs::config::StreamType::InputStream).await;
    stop_stream(common::structs::config::StreamType::OutputStream).await;

    stop_network_stream().await;

    // Delete the credential store keys
    let keys = vec![
        "gamertag",
        "gamerpic",
        "key_pk",
        "key_sk",
        "sig_pk",
        "sig_sk",
        "certificate",
        "key",
        "ca",
        "quic_connect_string",
        "host"
    ];
    for key in keys {
        del_credential(key.to_string()).await;
    }
}
