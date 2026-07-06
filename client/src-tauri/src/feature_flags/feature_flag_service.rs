use std::collections::HashMap;
use std::sync::Arc;
use std::sync::RwLock as StdRwLock;
use std::time::Duration;

use log::warn;
use open_feature::{EvaluationContext, OpenFeature};
use tokio::sync::{RwLock, watch};

use super::FlagsmithProvider;
use super::feature_flag::FeatureFlag;
use super::flagsmith::FlagsmithFlag;
use crate::analytics::AnalyticsService;
use crate::discord::DiscordTraitState;

pub struct FeatureFlagService {
    client: RwLock<Option<open_feature::Client>>,
    ready_tx: watch::Sender<bool>,
    ready_rx: watch::Receiver<bool>,
    api_key: String,
    server_url: String,
    install_id: String,
    build_number: i64,
    refresh_interval: Duration,
    http_client: reqwest::Client,
    // Shared with the live FlagsmithProvider so an on-demand refresh writes the
    // same cache the resolvers read. Populated in `initialize`.
    flag_cache: RwLock<Option<Arc<RwLock<HashMap<String, FlagsmithFlag>>>>>,
    normalized_url: RwLock<Option<String>>,
    // Shared with the FlagsmithProvider; carries the linked Discord roles that
    // become identity traits. Seeded at startup, updated on link/unlink.
    discord_state: Arc<StdRwLock<DiscordTraitState>>,
    // Optional so contexts without analytics (embedded bedrock server, tests)
    // construct the service without it. When present, every flag fetch emits a
    // FlagsmithFlagsFetched event.
    analytics: Option<Arc<AnalyticsService>>,
}

impl FeatureFlagService {
    pub fn new(
        api_key: String,
        server_url: String,
        install_id: String,
        build_number: i64,
        refresh_interval: Duration,
        analytics: Option<Arc<AnalyticsService>>,
    ) -> Self {
        let (ready_tx, ready_rx) = watch::channel(false);
        Self {
            client: RwLock::new(None),
            ready_tx,
            ready_rx,
            api_key,
            server_url,
            install_id,
            build_number,
            refresh_interval,
            http_client: super::flagsmith::pinned_client::FlagsmithPinnedClient::build(),
            flag_cache: RwLock::new(None),
            normalized_url: RwLock::new(None),
            discord_state: Arc::new(StdRwLock::new(DiscordTraitState::new())),
            analytics,
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
            self.build_number,
            self.refresh_interval,
            self.http_client.clone(),
            self.discord_state.clone(),
            self.analytics.clone(),
        );

        // Capture the provider's shared cache + normalized URL before it's
        // moved into OpenFeature, so `refresh` can re-fetch on demand.
        *self.flag_cache.write().await = Some(provider.cache());
        *self.normalized_url.write().await = Some(provider.server_url().to_string());

        let mut api = OpenFeature::singleton_mut().await;
        api.set_provider(provider).await;
        let ofe_client = api.create_client();

        let mut guard = self.client.write().await;
        *guard = Some(ofe_client);
        drop(guard);

        let _ = self.ready_tx.send(true);
    }

    // On-demand refresh of the flag cache (e.g. a debug "Refresh feature flags"
    // action). No-op when Flagsmith is disabled.
    pub async fn refresh(&self) -> Result<(), String> {
        if self.api_key.is_empty() {
            return Ok(());
        }
        let cache = self.flag_cache.read().await.clone();
        let url = self.normalized_url.read().await.clone();
        let roles = self.effective_roles_now();
        match (cache, url) {
            (Some(cache), Some(url)) => {
                let count = FlagsmithProvider::fetch_flags(
                    &self.http_client,
                    &url,
                    &self.api_key,
                    &self.install_id,
                    self.build_number,
                    &roles,
                    &cache,
                    self.analytics.as_ref(),
                )
                .await
                .map_err(|e| e.to_string())?;
                log::info!("Manually refreshed {} feature flags", count);
                Ok(())
            }
            _ => Err("Feature flags are not initialized yet".to_string()),
        }
    }

    fn now_secs() -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0)
    }

    fn effective_roles_now(&self) -> Vec<String> {
        self.current_effective_roles(Self::now_secs())
    }

    // Write the linked Discord roles without refreshing flags. Used at startup
    // before the provider initializes, so the first identity POST carries them.
    pub fn seed_discord_roles(&self, roles: Vec<String>, last_sync: Option<i64>) {
        if let Ok(mut s) = self.discord_state.write() {
            s.roles = roles;
            s.last_sync = last_sync;
        }
    }

    // Update the linked Discord roles and immediately re-fetch flags so gating
    // re-evaluates without a restart.
    pub async fn update_discord_roles(
        &self,
        roles: Vec<String>,
        last_sync: Option<i64>,
    ) -> Result<(), String> {
        self.seed_discord_roles(roles, last_sync);
        self.refresh().await
    }

    pub fn current_effective_roles(&self, now_secs: i64) -> Vec<String> {
        self.discord_state
            .read()
            .map(|s| s.effective_roles(now_secs))
            .unwrap_or_default()
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

    // Typed flag read. The `flag` value carries its own key + default; the
    // value type comes from `F::Value`. Boolean flags dispatch through
    // `is_enabled`; integer flags through `get_int_value`. Adding a new
    // value type means adding one `impl FlagsmithValue for …`, nothing
    // else changes at the service level.
    //
    // This is the entry point downstream code should prefer — it ties
    // flag definition (struct + trait impl) to call site through the type
    // system, preventing key typos and bool-vs-int mix-ups at compile time.
    pub async fn get<F: FeatureFlag>(&self, flag: F) -> F::Value {
        use super::flagsmith_value::FlagsmithValue;
        let key = flag.key();
        let default = flag.default();
        F::Value::fetch(self, key.as_ref(), default).await
    }

    // Returns the integer-valued flag from Flagsmith, or `None` if the flag
    // is unset / Flagsmith is unreachable / the client is not yet
    // initialized. Used by integer dials like
    // `feature.minecraft.max_trusted_protocol`.
    pub async fn get_int_value(&self, flag: &str) -> Option<i64> {
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
                client.get_int_value(flag, Some(&context), None).await.ok()
            }
            None => None,
        };
        match result {
            Some(v) => log::info!("Feature flag '{}' = {}", flag, v),
            None => log::info!("Feature flag '{}' = <unset>", flag),
        }
        result
    }
}
