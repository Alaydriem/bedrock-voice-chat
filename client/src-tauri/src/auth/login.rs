
use common::{
    ncryptflib::rocket::base64,
    structs::{ config::{ LoginRequest, LoginResponse }, ncryptf_json::JsonMessage },
};

use tauri_plugin_http::reqwest::{ header::{ HeaderMap, HeaderValue }, StatusCode };

use crate::auth::ncryptf;

pub(crate) async fn server_login(
    server: String,
    code: String,
    redirect: String
) -> Result<LoginResponse, bool> {
    let payload = LoginRequest { code: code.clone(), redirect_uri: redirect };

    // We're going to setup an ncryptf client
    let ek = match crate::auth::ncryptf::get_ek(server.clone()).await {
        Ok(ek) => ek,
        Err(e) => {
            log::error!("{:?}", e);
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

    let endpoint = format!("{}/{}", &server, ncryptf::AUTH_ENDPOINT);
    let client = crate::auth::ncryptf::get_reqwest_client();

    match client.post(endpoint.clone()).headers(headers).json(&payload).send().await {
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
                                                Some(data) => Ok(data),
                                                None => Err(false),
                                            }
                                        }
                                        Err(e) => {
                                            log::error!("{:?}", e.to_string());
                                            Err(false)
                                        }
                                    }
                                Err(e) => {
                                    log::error!("{}", e.to_string());
                                    return Err(false);
                                }
                            }
                        }
                        Err(e) => {
                            log::error!("{}", e.to_string());
                            return Err(false);
                        }
                    }
                }
                _ => Err(false)
            }
        }
        Err(e) => {
            log::error!("{}", e.to_string());
            Err(false)
        }
    }
}