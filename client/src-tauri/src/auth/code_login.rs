use common::request::CodeLoginRequest;
use common::response::LoginResponse;

use tauri_plugin_http::reqwest::{
    StatusCode,
    header::{HeaderMap, HeaderValue},
};

use base64::{Engine as _, engine::general_purpose};

use super::login::JsonMessage;

const CODE_AUTH_ENDPOINT: &str = "api/auth/code";

pub(crate) async fn code_login(
    server: String,
    gamertag: String,
    code: String,
) -> Result<LoginResponse, bool> {
    let payload = CodeLoginRequest { gamertag, code };

    let ek = match crate::auth::ncryptf::get_ek(server.clone()).await {
        Ok(ek) => ek,
        Err(e) => {
            log::error!("{:?}", e);
            return Err(false);
        }
    };

    let kp = common::ncryptflib::Keypair::new();

    let mut headers = HeaderMap::new();
    headers.insert(
        "Content-Type",
        HeaderValue::from_str("application/json").unwrap(),
    );
    headers.insert(
        "Accept",
        HeaderValue::from_str("application/vnd.ncryptf+json").unwrap(),
    );
    headers.insert("X-HashId", HeaderValue::from_str(&ek.hash_id).unwrap());
    headers.insert(
        "X-PubKey",
        HeaderValue::from_str(&general_purpose::STANDARD.encode(kp.get_public_key())).unwrap(),
    );

    let endpoint = format!("{}/{}", &server, CODE_AUTH_ENDPOINT);
    let client = crate::auth::ncryptf::get_reqwest_client();

    match client
        .post(endpoint.clone())
        .headers(headers)
        .json(&payload)
        .send()
        .await
    {
        Ok(response) => match response.status() {
            StatusCode::OK => match response.bytes().await {
                Ok(bytes) => {
                    let bbody = general_purpose::STANDARD.decode(bytes.clone()).unwrap();
                    let r = common::ncryptflib::Response::from(kp.get_secret_key()).unwrap();

                    match r.decrypt(bbody, None, None) {
                        Ok(json) => match serde_json::from_str::<JsonMessage<LoginResponse>>(&json)
                        {
                            Ok(response) => match response.data {
                                Some(data) => Ok(data),
                                None => Err(false),
                            },
                            Err(e) => {
                                log::error!("Response Error: {:?}", e.to_string());
                                Err(false)
                            }
                        },
                        Err(e) => {
                            log::error!("Ncryptf Error: {}", e.to_string());
                            Err(false)
                        }
                    }
                }
                Err(e) => {
                    log::error!("Error: {}", e.to_string());
                    Err(false)
                }
            },
            _ => {
                log::error!("Code login returned non-200 status");
                Err(false)
            }
        },
        Err(e) => {
            log::error!("Code login error: {}", e.to_string());
            Err(false)
        }
    }
}
