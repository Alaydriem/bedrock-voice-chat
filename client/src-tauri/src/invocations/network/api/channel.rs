use crate::invocations::{ network::client::Client, credentials::get_credential };
use reqwest::header::{ HeaderMap, HeaderValue };
use common::structs::channel::{ Channel, ChannelEvent, ChannelEvents };
use anyhow::anyhow;

const CHANNEL_ENDPOINT: &'static str = "/api/channel/";

#[tauri::command(async)]
pub(crate) async fn join_channel(id: String) -> Result<bool, bool> {
    let host = match get_host().await {
        Ok(host) => host,
        Err(_) => {
            return Err(false);
        }
    };

    let base_uri = format!("https://{}{}{}", host, CHANNEL_ENDPOINT, id);
    let mut client = match get_client().await {
        Ok(client) => client.put(base_uri),
        Err(_) => {
            return Err(false);
        }
    };
    let mut headers = HeaderMap::new();
    headers.append("Accept", HeaderValue::from_static("application/json"));
    headers.append("Content-Type", HeaderValue::from_static("application/json"));
    client = client.headers(headers);
    client = client.json(
        &(ChannelEvent {
            event: ChannelEvents::Join,
        })
    );

    match client.send().await {
        Ok(response) =>
            match response.json::<bool>().await {
                Ok(result) =>
                    match result {
                        true => Ok(true),
                        false => Err(false),
                    }
                Err(_) => Err(false),
            }
        Err(e) => {
            tracing::error!("{}", e.to_string());
            return Err(false);
        }
    }
}

#[tauri::command(async)]
pub(crate) async fn leave_channel(id: String) -> Result<bool, bool> {
    let host = match get_host().await {
        Ok(host) => host,
        Err(_) => {
            return Err(false);
        }
    };

    let base_uri = format!("https://{}{}{}", host, CHANNEL_ENDPOINT, id);
    let mut client = match get_client().await {
        Ok(client) => client.put(base_uri),
        Err(_) => {
            return Err(false);
        }
    };
    let mut headers = HeaderMap::new();
    headers.append("Accept", HeaderValue::from_static("application/json"));
    headers.append("Content-Type", HeaderValue::from_static("application/json"));
    client = client.headers(headers);
    client = client.json(
        &(ChannelEvent {
            event: ChannelEvents::Leave,
        })
    );

    match client.send().await {
        Ok(response) =>
            match response.json::<bool>().await {
                Ok(result) =>
                    match result {
                        true => Ok(true),
                        false => Err(false),
                    }
                Err(_) => Err(false),
            }
        Err(e) => {
            tracing::error!("{}", e.to_string());
            return Err(false);
        }
    }
}

#[tauri::command(async)]
pub(crate) async fn get_channels(id: Option<String>) -> Result<Vec<Channel>, bool> {
    let host = match get_host().await {
        Ok(host) => host,
        Err(_) => {
            return Err(false);
        }
    };

    let base_uri = format!("https://{}{}{}", host, CHANNEL_ENDPOINT, match id {
        Some(id) => format!("?{}", id),
        None => format!(""),
    });

    let mut client = match get_client().await {
        Ok(client) => client.get(base_uri),
        Err(_) => {
            return Err(false);
        }
    };
    let mut headers = HeaderMap::new();
    headers.append("Accept", HeaderValue::from_static("application/json"));
    headers.append("Content-Type", HeaderValue::from_static("application/json"));
    client = client.headers(headers);

    match client.send().await {
        Ok(response) =>
            match response.json::<Vec<Channel>>().await {
                Ok(result) => {
                    tracing::info!("{:?}", result);
                    Ok(result)
                }
                Err(_) => { Err(false) }
            }
        Err(e) => {
            tracing::error!("{}", e.to_string());
            return Err(false);
        }
    }
}

#[tauri::command(async)]
pub(crate) async fn create_channel(name: String) -> Result<Channel, bool> {
    let host = match get_host().await {
        Ok(host) => host,
        Err(_) => {
            return Err(false);
        }
    };

    let base_uri = format!("https://{}{}", host, CHANNEL_ENDPOINT);
    let mut client = match get_client().await {
        Ok(client) => client.post(base_uri),
        Err(_) => {
            return Err(false);
        }
    };
    let mut headers = HeaderMap::new();
    headers.append("Accept", HeaderValue::from_static("application/json"));
    headers.append("Content-Type", HeaderValue::from_static("application/json"));
    client = client.headers(headers);
    client = client.json(&name);

    match client.send().await {
        Ok(response) =>
            match response.json::<serde_json::Value>().await {
                Ok(result) =>
                    match get_channels(Some(result.to_string())).await {
                        Ok(channels) => {
                            for channel in channels {
                                if channel.id().eq(&result) {
                                    return Ok(channel);
                                }
                            }

                            Err(false)
                        }
                        Err(_) => { Err(false) }
                    }
                Err(_) => { Err(false) }
            }
        Err(_) => {
            return Err(false);
        }
    }
}

#[tauri::command(async)]
pub(crate) async fn delete_channel(id: String) -> Result<bool, bool> {
    let host = match get_host().await {
        Ok(host) => host,
        Err(_) => {
            return Err(false);
        }
    };

    let base_uri = format!("https://{}{}/{}", host, CHANNEL_ENDPOINT, id);
    let mut client = match get_client().await {
        Ok(client) => client.delete(base_uri),
        Err(_) => {
            return Err(false);
        }
    };
    let mut headers = HeaderMap::new();
    headers.append("Accept", HeaderValue::from_static("application/json"));
    headers.append("Content-Type", HeaderValue::from_static("application/json"));
    client = client.headers(headers);

    match client.send().await {
        Ok(response) =>
            match response.json::<bool>().await {
                Ok(result) =>
                    match result {
                        true => Ok(true),
                        false => Err(false),
                    }
                Err(_) => Err(false),
            }
        Err(e) => {
            tracing::error!("{}", e.to_string());
            return Err(false);
        }
    }
}

/// Returns the host
async fn get_host() -> Result<String, anyhow::Error> {
    match get_credential("host") {
        Ok(host) => Ok(format!("{}", host)),
        Err(_) => { Err(anyhow!("Missing host endpoint.")) }
    }
}

/// Returns a workable reqwest client to call the API with mTLS client credentials
async fn get_client() -> Result<reqwest::Client, anyhow::Error> {
    match Client::new().await {
        Ok(client) => Ok(client.get_reqwest_client().await),
        Err(e) => {
            tracing::error!("{}", e.to_string());
            return Err(anyhow!("Could not retrieve client."));
        }
    }
}

#[cfg(test)]
mod test_channel_api {
    use super::*;

    #[tokio::test]
    async fn channel_management() {
        let channel = create_channel("foo".to_string()).await.unwrap();
        let channel_id = channel.id();
        let result = join_channel(channel_id.clone()).await;
        assert_eq!(result, Ok(true));
        let result = leave_channel(channel_id.clone()).await;
        assert_eq!(result, Ok(true));
        assert_eq!(Ok(true), delete_channel(channel_id.clone()).await);
    }
}
