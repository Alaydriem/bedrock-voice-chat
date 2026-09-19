use anyhow::anyhow;
use base64::{Engine as _, engine::general_purpose};
use common::request::{CodeLoginRequest, LoginRequest};
use common::response::ApiConfigResponse;
use common::response::{JsonMessage, LoginResponse};
use tauri_plugin_http::reqwest::{
    StatusCode,
    header::{HeaderMap, HeaderValue},
};

use crate::auth::LoginFailure;
use crate::auth::NcryptfClient;
use common::structs::reachability::AddressFamilyPreference;

pub struct AuthClient;

impl AuthClient {
    const CODE_AUTH_ENDPOINT: &'static str = "api/auth/code";

    /// The unauthenticated `/api/config` the sign-in screen reads to learn the server's
    /// OAuth client id.
    ///
    /// Uses the login client so the screen's verdict is measured over the same transport as
    /// the sign-in it precedes.
    pub(crate) async fn fetch_public_config(
        server: String,
        preference: AddressFamilyPreference,
    ) -> Result<ApiConfigResponse, anyhow::Error> {
        let endpoint =
            super::ServerEndpoint::join(&server, super::ncryptf_client::CONFIG_ENDPOINT);
        let client = NcryptfClient::get_reqwest_client(preference);

        let response = match client.get(&endpoint).send().await {
            Ok(response) => response,
            Err(e) => {
                let message = LoginFailure::describe(&endpoint, &e);
                log::error!("{}", message);
                return Err(anyhow!(message));
            }
        };

        match response.status() {
            StatusCode::OK => response.json::<ApiConfigResponse>().await.map_err(|e| {
                anyhow!(
                    "{} answered with a configuration this build cannot read: {}",
                    endpoint,
                    e
                )
            }),
            StatusCode::FORBIDDEN => Err(anyhow!("403 Forbidden: the server refused this client")),
            status => Err(anyhow!("{} returned HTTP {}", endpoint, status)),
        }
    }

    pub(crate) async fn server_login(
        server: String,
        code: String,
        redirect: String,
        preference: AddressFamilyPreference,
    ) -> Result<LoginResponse, anyhow::Error> {
        let payload = LoginRequest {
            code: code.clone(),
            redirect_uri: redirect,
        };

        // We're going to setup an ncryptf client
        let ek = match NcryptfClient::get_ek(server.clone(), preference).await {
            Ok(ek) => ek,
            Err(e) => {
                log::error!("{}", LoginFailure::describe(&server, &*e));
                return Err(anyhow!("Unable to reach the server for key exchange"));
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

        let endpoint = super::ServerEndpoint::join(&server, super::ncryptf_client::AUTH_ENDPOINT);
        let client = NcryptfClient::get_reqwest_client(preference);

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
                            Ok(json) => {
                                match serde_json::from_str::<JsonMessage<LoginResponse>>(&json) {
                                    Ok(response) => match response.data {
                                        Some(data) => Ok(data),
                                        None => Err(anyhow!("Login response contained no data")),
                                    },
                                    Err(e) => {
                                        log::error!("Response Error: {:?}", e.to_string());
                                        Err(anyhow!("Login response could not be parsed"))
                                    }
                                }
                            }
                            Err(e) => {
                                log::error!("Ncryptf Error: {}", e.to_string());
                                return Err(anyhow!("Login response could not be decrypted"));
                            }
                        }
                    }
                    Err(e) => {
                        log::error!("Error: {}", e.to_string());
                        return Err(anyhow!("Login response could not be read"));
                    }
                },
                // The server answers 403 when the account authenticated with Xbox Live
                // but is not authorized on this server (not registered, or banished).
                StatusCode::FORBIDDEN => Err(anyhow!(
                    "403 Forbidden: the server denied access for this account"
                )),
                // The sign-in itself did not complete: the authorization code was spent,
                // expired, or refused. Retryable, and nothing to do with the account — so the
                // wording avoids the words the caller reads as a denial.
                StatusCode::UNAUTHORIZED => Err(anyhow!(
                    "401 Unauthorized: the sign-in did not complete; please sign in again"
                )),
                // Upstream identity providers, not us and not the account.
                StatusCode::BAD_GATEWAY => Err(anyhow!(
                    "502 Bad Gateway: Xbox Live could not be reached; please try again"
                )),
                status => Err(anyhow!("Login failed: server returned HTTP {}", status)),
            },
            Err(e) => {
                let message = LoginFailure::describe(&endpoint, &e);
                log::error!("{}", message);
                Err(anyhow!(message))
            }
        }
    }

    pub(crate) async fn code_login(
        server: String,
        code: String,
        preference: AddressFamilyPreference,
    ) -> Result<LoginResponse, bool> {
        let payload = CodeLoginRequest { code };

        let ek = match NcryptfClient::get_ek(server.clone(), preference).await {
            Ok(ek) => ek,
            Err(e) => {
                log::error!("{}", LoginFailure::describe(&server, &*e));
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

        let endpoint = super::ServerEndpoint::join(&server, Self::CODE_AUTH_ENDPOINT);
        let client = NcryptfClient::get_reqwest_client(preference);

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
                            Ok(json) => {
                                match serde_json::from_str::<JsonMessage<LoginResponse>>(&json) {
                                    Ok(response) => match response.data {
                                        Some(data) => Ok(data),
                                        None => Err(false),
                                    },
                                    Err(e) => {
                                        log::error!("Response Error: {:?}", e.to_string());
                                        Err(false)
                                    }
                                }
                            }
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
                log::error!("{}", LoginFailure::describe(&endpoint, &e));
                Err(false)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_server_login() {
        let server = "https://local.bedrockvc.stream".to_string();
        let code = "test_code".to_string();
        let redirect = "http://localhost:3000/redirect".to_string();

        let result = AuthClient::server_login(
            server,
            code,
            redirect,
            AddressFamilyPreference::PreferIpv4,
        )
        .await;

        assert!(result.is_err(), "Expected login to fail for test data");
    }
}
