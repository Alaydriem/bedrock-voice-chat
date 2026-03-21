use std::time::Duration;

use log::warn;
use open_feature::{EvaluationContext, OpenFeature};
use tokio::sync::{watch, RwLock};

use super::FlagsmithProvider;

pub struct FeatureFlagService {
    client: RwLock<Option<open_feature::Client>>,
    ready_tx: watch::Sender<bool>,
    ready_rx: watch::Receiver<bool>,
    api_key: String,
    server_url: String,
    install_id: String,
    refresh_interval: Duration,
}

impl FeatureFlagService {
    pub fn new(api_key: String, server_url: String, install_id: String, refresh_interval: Duration) -> Self {
        let (ready_tx, ready_rx) = watch::channel(false);
        Self {
            client: RwLock::new(None),
            ready_tx,
            ready_rx,
            api_key,
            server_url,
            install_id,
            refresh_interval,
        }
    }

    pub async fn initialize(&self) {
        if self.api_key.is_empty() {
            warn!("FLAGSMITH_KEY not set, feature flags disabled");
            let _ = self.ready_tx.send(true);
            return;
        }

        let provider = FlagsmithProvider::new(
            self.api_key.clone(),
            self.server_url.clone(),
            self.install_id.clone(),
            self.refresh_interval,
        );

        let mut api = OpenFeature::singleton_mut().await;
        api.set_provider(provider).await;
        let ofe_client = api.create_client();

        let mut guard = self.client.write().await;
        *guard = Some(ofe_client);
        drop(guard);

        let _ = self.ready_tx.send(true);
    }

    pub async fn is_enabled(&self, flag: &str) -> bool {
        let mut rx = self.ready_rx.clone();
        while !*rx.borrow_and_update() {
            if rx.changed().await.is_err() {
                break;
            }
        }

        let guard = self.client.read().await;
        let result = match guard.as_ref() {
            Some(client) => {
                let mut context = EvaluationContext::default();
                context.targeting_key = Some(self.install_id.clone());
                client
                    .get_bool_value(flag, Some(&context), None)
                    .await
                    .unwrap_or(false)
            }
            None => false,
        };
        log::info!("Feature flag '{}' = {}", flag, result);
        result
    }
}
