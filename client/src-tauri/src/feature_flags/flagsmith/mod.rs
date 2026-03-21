pub(crate) mod feature;
pub(crate) mod flag;
pub(crate) mod identity_response;
pub(crate) mod value;

pub(crate) use flag::FlagsmithFlag;
pub(crate) use value::FlagsmithFlagValue;

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use log::{info, warn};
use open_feature::provider::{FeatureProvider, ProviderMetadata, ResolutionDetails};
use open_feature::{EvaluationContext, EvaluationError, EvaluationErrorCode, EvaluationResult, StructValue, Value};
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
    pub fn new(api_key: String, server_url: String, install_id: String, refresh_interval: Duration) -> Self {
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
            http_client: reqwest::Client::new(),
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    async fn refresh(&self) -> Result<(), anyhow::Error> {
        let url = format!("{}identities/?identifier={}", self.server_url, self.install_id);
        let response = self
            .http_client
            .get(&url)
            .header("X-Environment-Key", &self.api_key)
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
        let mut cache = self.cache.write().await;
        cache.clear();
        for flag in identity_response.flags {
            info!("Flag '{}': enabled={}, value={:?}", flag.feature.name, flag.enabled, flag.value);
            cache.insert(flag.feature.name.clone(), flag);
        }
        info!("Refreshed {} feature flags for identity {}", cache.len(), self.install_id);
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
                let url = format!("{}identities/?identifier={}", server_url, install_id);
                match http_client
                    .get(&url)
                    .header("X-Environment-Key", &api_key)
                    .header("Content-Type", "application/json")
                    .send()
                    .await
                {
                    Ok(response) if response.status().is_success() => {
                        match response.json::<FlagsmithIdentityResponse>().await {
                            Ok(identity_response) => {
                                let mut c = cache.write().await;
                                c.clear();
                                for flag in identity_response.flags {
                                    c.insert(flag.feature.name.clone(), flag);
                                }
                                info!("Refreshed {} feature flags", c.len());
                            }
                            Err(e) => warn!("Feature flag refresh parse failed: {}", e),
                        }
                    }
                    Ok(response) => {
                        warn!("Feature flag refresh returned status {}", response.status());
                    }
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
            Some(flag) => Ok(ResolutionDetails::new(flag.enabled)),
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
