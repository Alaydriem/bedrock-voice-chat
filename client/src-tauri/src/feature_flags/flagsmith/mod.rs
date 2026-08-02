pub(crate) mod feature;
pub(crate) mod flag;
pub(crate) mod identity_response;
pub(crate) mod pinned_client;
pub(crate) mod value;

pub(crate) use flag::FlagsmithFlag;
pub(crate) use value::FlagsmithFlagValue;

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::RwLock as StdRwLock;
use std::time::Duration;

use async_trait::async_trait;
use log::{info, warn};
use open_feature::provider::{FeatureProvider, ProviderMetadata, ResolutionDetails};
use open_feature::{
    EvaluationContext, EvaluationError, EvaluationErrorCode, EvaluationResult, StructValue, Value,
};
use tokio::sync::{RwLock, watch};

use self::identity_response::FlagsmithIdentityResponse;
use crate::analytics::AnalyticsService;
use crate::discord::DiscordTraitState;
use common::structs::{AnalyticsEvent, AnalyticsEventData};

pub struct FlagsmithProvider {
    metadata: ProviderMetadata,
    api_key: String,
    server_url: String,
    install_id: String,
    build_number: i64,
    refresh_interval: Duration,
    http_client: reqwest::Client,
    cache: Arc<RwLock<HashMap<String, FlagsmithFlag>>>,
    discord_state: Arc<StdRwLock<DiscordTraitState>>,
    analytics: Option<Arc<AnalyticsService>>,
    generation: watch::Sender<u64>,
}

impl FlagsmithProvider {
    pub fn new(
        api_key: String,
        server_url: String,
        install_id: String,
        build_number: i64,
        refresh_interval: Duration,
        http_client: reqwest::Client,
        discord_state: Arc<StdRwLock<DiscordTraitState>>,
        analytics: Option<Arc<AnalyticsService>>,
        generation: watch::Sender<u64>,
    ) -> Self {
        let normalized_url = if server_url.ends_with("/api/v1/") {
            server_url
        } else if server_url.ends_with('/') {
            format!("{}api/v1/", server_url)
        } else {
            format!("{}/api/v1/", server_url)
        };

        Self {
            metadata: ProviderMetadata::new("flagsmith"),
            api_key,
            server_url: normalized_url,
            install_id,
            build_number,
            refresh_interval,
            http_client,
            cache: Arc::new(RwLock::new(HashMap::new())),
            discord_state,
            analytics,
            generation,
        }
    }

    // Snapshot the currently-effective Discord role IDs (30-day rule applied),
    // dropping the read guard before any await.
    fn effective_roles_now(discord_state: &StdRwLock<DiscordTraitState>) -> Vec<String> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        discord_state
            .read()
            .map(|s| s.effective_roles(now))
            .unwrap_or_default()
    }

    pub(crate) fn cache(&self) -> Arc<RwLock<HashMap<String, FlagsmithFlag>>> {
        self.cache.clone()
    }

    pub(crate) fn server_url(&self) -> &str {
        &self.server_url
    }

    pub fn build_identity_body(
        install_id: &str,
        build_number: i64,
        discord_roles: &[String],
    ) -> serde_json::Value {
        let mut traits = vec![serde_json::json!({
            "trait_key": "build_number",
            "trait_value": build_number,
            "transient": true
        })];
        for role in discord_roles {
            traits.push(serde_json::json!({
                "trait_key": format!("discord-role-{}", role),
                "trait_value": true,
                "transient": true
            }));
        }
        for label in crate::discord::RoleCategory::labels_for(discord_roles) {
            traits.push(serde_json::json!({
                "trait_key": format!("discord-role-{}", label),
                "trait_value": true,
                "transient": true
            }));
        }
        serde_json::json!({ "identifier": install_id, "traits": traits })
    }

    // Fetch the identity's flags and replace the cache. Shared by the initial
    // load, the background poll loop, and on-demand refresh — all of which
    // write the same `Arc<RwLock<..>>` the resolvers read. Sends `build_number`
    // as a transient identity trait so flags can gate on the client build.
    // Bumping `generation` here rather than at each call site is what makes
    // every refresh path observable to a watcher, including the background
    // poll.
    pub(crate) async fn fetch_flags(
        http_client: &reqwest::Client,
        server_url: &str,
        api_key: &str,
        install_id: &str,
        build_number: i64,
        discord_roles: &[String],
        cache: &RwLock<HashMap<String, FlagsmithFlag>>,
        analytics: Option<&Arc<AnalyticsService>>,
        generation: &watch::Sender<u64>,
    ) -> Result<usize, anyhow::Error> {
        let url = format!("{}identities/", server_url);

        let trait_summary: Vec<String> = std::iter::once(format!("build_number={}", build_number))
            .chain(discord_roles.iter().map(|r| format!("discord-role-{}", r)))
            .collect();
        info!(
            "Flagsmith identity POST: id={} traits=[{}]",
            install_id,
            trait_summary.join(", ")
        );

        let response = http_client
            .post(&url)
            .header("X-Environment-Key", api_key)
            .header("Content-Type", "application/json")
            .json(&Self::build_identity_body(install_id, build_number, discord_roles))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Flagsmith API returned status {}",
                response.status()
            ));
        }

        let identity_response: FlagsmithIdentityResponse = response.json().await?;
        let mut c = cache.write().await;
        c.clear();
        for flag in identity_response.flags {
            info!(
                "Flag '{}': enabled={}, value={:?}",
                flag.feature.name, flag.enabled, flag.value
            );
            c.insert(flag.feature.name.clone(), flag);
        }
        let count = c.len();
        drop(c);
        generation.send_modify(|g| *g = g.wrapping_add(1));
        if let Some(analytics) = analytics {
            analytics.track(
                AnalyticsEvent::FlagsmithFlagsFetched,
                Some(AnalyticsEventData::new().insert("count", count as i64)),
            );
        }
        Ok(count)
    }

    async fn refresh(&self) -> Result<(), anyhow::Error> {
        let roles = Self::effective_roles_now(&self.discord_state);
        let count = Self::fetch_flags(
            &self.http_client,
            &self.server_url,
            &self.api_key,
            &self.install_id,
            self.build_number,
            &roles,
            &self.cache,
            self.analytics.as_ref(),
            &self.generation,
        )
        .await?;
        info!(
            "Refreshed {} feature flags for identity {}",
            count, self.install_id
        );
        Ok(())
    }

    fn flag_not_found(flag_key: &str) -> EvaluationError {
        EvaluationError::builder()
            .code(EvaluationErrorCode::FlagNotFound)
            .message(format!("Flag '{}' not found", flag_key))
            .build()
    }

    fn no_value(flag_key: &str) -> EvaluationError {
        EvaluationError::builder()
            .code(EvaluationErrorCode::FlagNotFound)
            .message(format!("Flag '{}' has no value", flag_key))
            .build()
    }

    fn type_mismatch(flag_key: &str, expected: &str) -> EvaluationError {
        EvaluationError::builder()
            .code(EvaluationErrorCode::TypeMismatch)
            .message(format!("Flag '{}' value is not {}", flag_key, expected))
            .build()
    }
}

#[async_trait]
impl FeatureProvider for FlagsmithProvider {
    async fn initialize(&mut self, _context: &EvaluationContext) {
        if let Err(e) = self.refresh().await {
            warn!("Initial feature flag refresh failed: {}", e);
        }

        let cache = self.cache.clone();
        let http_client = self.http_client.clone();
        let api_key = self.api_key.clone();
        let server_url = self.server_url.clone();
        let install_id = self.install_id.clone();
        let build_number = self.build_number;
        let refresh_interval = self.refresh_interval;
        let discord_state = self.discord_state.clone();
        let analytics = self.analytics.clone();
        let generation = self.generation.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(refresh_interval);
            interval.tick().await;
            loop {
                interval.tick().await;
                let roles = FlagsmithProvider::effective_roles_now(&discord_state);
                match FlagsmithProvider::fetch_flags(
                    &http_client,
                    &server_url,
                    &api_key,
                    &install_id,
                    build_number,
                    &roles,
                    &cache,
                    analytics.as_ref(),
                    &generation,
                )
                .await
                {
                    Ok(n) => info!("Refreshed {} feature flags", n),
                    Err(e) => warn!("Feature flag refresh failed: {}", e),
                }
            }
        });
    }

    fn metadata(&self) -> &ProviderMetadata {
        &self.metadata
    }

    async fn resolve_bool_value(
        &self,
        flag_key: &str,
        _evaluation_context: &EvaluationContext,
    ) -> EvaluationResult<ResolutionDetails<bool>> {
        let cache = self.cache.read().await;
        match cache.get(flag_key) {
            // A boolean flag resolves to its value, gated by the enabled
            // toggle. Disabled -> false. Enabled with an explicit value ->
            // that value (so an enabled flag whose value is false is false).
            // Enabled with no value -> true (a bare on-toggle).
            Some(flag) => {
                let result = flag.enabled
                    && match &flag.value {
                        Some(FlagsmithFlagValue::Bool(b)) => *b,
                        Some(FlagsmithFlagValue::String(s)) => {
                            !matches!(s.trim().to_ascii_lowercase().as_str(), "false" | "0" | "")
                        }
                        Some(FlagsmithFlagValue::Int(i)) => *i != 0,
                        Some(FlagsmithFlagValue::Float(f)) => *f != 0.0,
                        Some(FlagsmithFlagValue::Json(_)) => true,
                        None => true,
                    };
                Ok(ResolutionDetails::new(result))
            }
            None => Err(Self::flag_not_found(flag_key)),
        }
    }

    async fn resolve_int_value(
        &self,
        flag_key: &str,
        _evaluation_context: &EvaluationContext,
    ) -> EvaluationResult<ResolutionDetails<i64>> {
        let cache = self.cache.read().await;
        match cache.get(flag_key) {
            Some(flag) => match &flag.value {
                Some(FlagsmithFlagValue::Int(i)) => Ok(ResolutionDetails::new(*i)),
                Some(_) => Err(Self::type_mismatch(flag_key, "an integer")),
                None => Err(Self::no_value(flag_key)),
            },
            None => Err(Self::flag_not_found(flag_key)),
        }
    }

    async fn resolve_float_value(
        &self,
        flag_key: &str,
        _evaluation_context: &EvaluationContext,
    ) -> EvaluationResult<ResolutionDetails<f64>> {
        let cache = self.cache.read().await;
        match cache.get(flag_key) {
            Some(flag) => match &flag.value {
                Some(FlagsmithFlagValue::Float(f)) => Ok(ResolutionDetails::new(*f)),
                Some(FlagsmithFlagValue::Int(i)) => Ok(ResolutionDetails::new(*i as f64)),
                Some(_) => Err(Self::type_mismatch(flag_key, "a float")),
                None => Err(Self::no_value(flag_key)),
            },
            None => Err(Self::flag_not_found(flag_key)),
        }
    }

    async fn resolve_string_value(
        &self,
        flag_key: &str,
        _evaluation_context: &EvaluationContext,
    ) -> EvaluationResult<ResolutionDetails<String>> {
        let cache = self.cache.read().await;
        match cache.get(flag_key) {
            Some(flag) => match &flag.value {
                Some(FlagsmithFlagValue::String(s)) => Ok(ResolutionDetails::new(s.clone())),
                Some(FlagsmithFlagValue::Bool(b)) => Ok(ResolutionDetails::new(b.to_string())),
                Some(FlagsmithFlagValue::Int(i)) => Ok(ResolutionDetails::new(i.to_string())),
                Some(FlagsmithFlagValue::Float(f)) => Ok(ResolutionDetails::new(f.to_string())),
                Some(FlagsmithFlagValue::Json(v)) => Ok(ResolutionDetails::new(v.to_string())),
                None => Err(Self::no_value(flag_key)),
            },
            None => Err(Self::flag_not_found(flag_key)),
        }
    }

    async fn resolve_struct_value(
        &self,
        flag_key: &str,
        _evaluation_context: &EvaluationContext,
    ) -> EvaluationResult<ResolutionDetails<StructValue>> {
        let cache = self.cache.read().await;
        match cache.get(flag_key) {
            Some(flag) => match &flag.value {
                Some(val) => match val.to_of_value() {
                    Value::Struct(sv) => Ok(ResolutionDetails::new(sv)),
                    _ => Err(Self::type_mismatch(flag_key, "a struct")),
                },
                None => Err(Self::no_value(flag_key)),
            },
            None => Err(Self::flag_not_found(flag_key)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::feature::FlagsmithFeature;
    use super::*;

    fn provider() -> FlagsmithProvider {
        FlagsmithProvider::new(
            String::new(),
            String::new(),
            String::new(),
            0,
            Duration::from_secs(3600),
            reqwest::Client::new(),
            std::sync::Arc::new(std::sync::RwLock::new(crate::discord::DiscordTraitState::new())),
            None,
            watch::channel(0u64).0,
        )
    }

    fn flag(enabled: bool, value: Option<FlagsmithFlagValue>) -> FlagsmithFlag {
        FlagsmithFlag {
            enabled,
            feature: FlagsmithFeature {
                name: "k".to_string(),
            },
            value,
        }
    }

    async fn resolve(p: &FlagsmithProvider, key: &str) -> bool {
        p.resolve_bool_value(key, &EvaluationContext::default())
            .await
            .unwrap()
            .value
    }

    #[tokio::test]
    async fn bool_resolves_value_gated_by_enabled() {
        let p = provider();
        {
            let mut c = p.cache.write().await;
            c.insert(
                "enabled_true".into(),
                flag(true, Some(FlagsmithFlagValue::Bool(true))),
            );
            c.insert(
                "enabled_false".into(),
                flag(true, Some(FlagsmithFlagValue::Bool(false))),
            );
            c.insert(
                "disabled_with_true_value".into(),
                flag(false, Some(FlagsmithFlagValue::Bool(true))),
            );
            c.insert("enabled_no_value".into(), flag(true, None));
        }

        // Enabled + value true -> true.
        assert!(resolve(&p, "enabled_true").await);
        // Enabled + value false -> false (the bug this fixes: was true).
        assert!(!resolve(&p, "enabled_false").await);
        // Disabled gates off regardless of value.
        assert!(!resolve(&p, "disabled_with_true_value").await);
        // Bare enabled toggle with no value -> true.
        assert!(resolve(&p, "enabled_no_value").await);
    }
}
