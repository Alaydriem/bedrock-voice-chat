use crate::analytics::aptabase::{Event, SystemProps};
use crate::analytics::dtos::QueuedEvent;

pub struct Provider {
    client: reqwest::Client,
    host: String,
    app_key: String,
    system_props: SystemProps,
}

impl Provider {
    pub fn new(host: String, app_key: String) -> Self {
        let app_version = env!("CARGO_PKG_VERSION").to_string();
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .unwrap_or_default(),
            host,
            app_key,
            system_props: SystemProps::new(app_version),
        }
    }

    pub async fn send_batch(
        &self,
        events: &[QueuedEvent],
        install_id: &str,
    ) -> Result<(), anyhow::Error> {
        if events.is_empty() {
            return Ok(());
        }

        let url = format!("{}/api/v0/events", self.host);
        let user_agent = format!(
            "{}/{} bvc/{}",
            std::env::consts::OS,
            std::env::consts::ARCH,
            self.system_props.app_version
        );

        for chunk in events.chunks(25) {
            let body: Vec<Event> = chunk
                .iter()
                .map(|e| {
                    let mut map: serde_json::Map<String, serde_json::Value> = match &e.properties {
                        Some(data) => data.properties.iter()
                            .map(|(k, v)| (k.clone(), v.clone()))
                            .collect(),
                        None => serde_json::Map::new(),
                    };
                    map.insert(
                        "install_id".to_string(),
                        serde_json::Value::String(install_id.to_string()),
                    );

                    Event {
                        timestamp: e.timestamp,
                        session_id: install_id.to_string(),
                        event_name: e.event.name().to_string(),
                        system_props: self.system_props.clone(),
                        props: serde_json::Value::Object(map),
                    }
                })
                .collect();

            let response = self
                .client
                .post(&url)
                .header("Content-Type", "application/json")
                .header("App-Key", &self.app_key)
                .header("User-Agent", &user_agent)
                .json(&body)
                .send()
                .await;

            match response {
                Ok(resp) => {
                    let status = resp.status();
                    if status.is_server_error() {
                        log::warn!("Aptabase server error: {}. Continuing with remaining chunks.", status);
                    } else if status.is_client_error() {
                        log::warn!("Aptabase client error: {}. Events discarded.", status);
                    }
                }
                Err(e) => {
                    log::warn!("Aptabase request failed: {}. Continuing with remaining chunks.", e);
                }
            }
        }

        Ok(())
    }
}
