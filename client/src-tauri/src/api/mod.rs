pub(crate) mod commands;
use log::error;
mod channel;
mod client;

use tauri_plugin_http::reqwest::{
    Client as ReqwestClient,
    StatusCode,
    header::{ HeaderMap, HeaderValue}
};

pub(crate) struct Api {
    endpoint: String,
    client: client::Client
}

impl Api {
    pub fn new(endpoint: String, ca_cert: String, pem: String) -> Self {
        Self {
            endpoint,
            client: client::Client::new(ca_cert, pem)
        }
    }

    async fn get_client(&self) -> ReqwestClient {
        self.client.get_client().await
    }

    pub(crate) async fn ping(&self) -> Result<(), bool> {
        let client = self.get_client().await;
        let mut headers = HeaderMap::new();
        headers.insert(
            "Content-Type",
            HeaderValue::from_str("application/json").unwrap(),
        );
        headers.insert(
            "Accept",
            HeaderValue::from_str("application/json").unwrap(),
        );

        match client.get(format!("{}/api/ping", self.endpoint)).headers(headers).send().await {
            Ok(response) => match response.status() {
                StatusCode::OK => Ok(()),
                _ => Err(false),
            },
            Err(e) => {
                error!("{}", e);
                Err(false)
            }
        }
    }
}