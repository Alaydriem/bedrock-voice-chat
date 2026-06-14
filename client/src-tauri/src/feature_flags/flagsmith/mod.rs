pub(crate) mod feature;
pub(crate) mod flag;
pub(crate) mod identity_response;
pub(crate) mod spki_pinning_verifier;
pub(crate) mod value;

pub(crate) use flag::FlagsmithFlag;
pub(crate) use value::FlagsmithFlagValue;

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use log::{info, warn};
use open_feature::provider::{FeatureProvider, ProviderMetadata, ResolutionDetails};
use open_feature::{
    EvaluationContext, EvaluationError, EvaluationErrorCode, EvaluationResult, StructValue, Value,
};
use tokio::sync::RwLock;

use self::identity_response::FlagsmithIdentityResponse;

pub struct FlagsmithProvider {
    metadata: ProviderMetadata,
    api_key: String,
    server_url: String,
    install_id: String,
    refresh_interval: Duration,
    http_client: reqwest::Client,
    cache: Arc<RwLock<HashMap<String, FlagsmithFlag>>>,
}

impl FlagsmithProvider {
    pub fn new(
        api_key: String,
        server_url: String,
        install_id: String,
        refresh_interval: Duration,
        http_client: reqwest::Client,
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
            refresh_interval,
            http_client,
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub(crate) fn cache(&self) -> Arc<RwLock<HashMap<String, FlagsmithFlag>>> {
        self.cache.clone()
    }

    pub(crate) fn server_url(&self) -> &str {
        &self.server_url
    }

    // Fetch the identity's flags and replace the cache. Shared by the initial
    // load, the background poll loop, and on-demand refresh — all of which
    // write the same `Arc<RwLock<..>>` the resolvers read.
    pub(crate) async fn fetch_flags(
        http_client: &reqwest::Client,
        server_url: &str,
        api_key: &str,
        install_id: &str,
        cache: &RwLock<HashMap<String, FlagsmithFlag>>,
    ) -> Result<usize, anyhow::Error> {
        let url = format!("{}identities/?identifier={}", server_url, install_id);
        let response = http_client
            .get(&url)
            .header("X-Environment-Key", api_key)
            .header("Content-Type", "application/json")
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
        Ok(c.len())
    }

    async fn refresh(&self) -> Result<(), anyhow::Error> {
        let count = Self::fetch_flags(
            &self.http_client,
            &self.server_url,
            &self.api_key,
            &self.install_id,
            &self.cache,
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
        let refresh_interval = self.refresh_interval;

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(refresh_interval);
            interval.tick().await;
            loop {
                interval.tick().await;
                match FlagsmithProvider::fetch_flags(
                    &http_client,
                    &server_url,
                    &api_key,
                    &install_id,
                    &cache,
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
            Duration::from_secs(3600),
            reqwest::Client::new(),
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
