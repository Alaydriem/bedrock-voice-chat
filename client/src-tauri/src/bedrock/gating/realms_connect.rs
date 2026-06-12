use std::sync::Arc;

use common::structs::iap::{GateReason, RealmsGateStatus};
use common::structs::{AnalyticsEvent, AnalyticsEventData};

use crate::analytics::AnalyticsService;
use crate::feature_flags::FeatureFlagService;
use crate::feature_flags::flags::bedrock::{FreeWeekendEnabled, RealmsConnectEnabled};

// Decides whether Realms Connect may run:
//   feature_enabled AND (free_weekend OR entitled)
// Emits one analytics event per evaluation. `is_entitled` is supplied by the
// caller as a cached read.
pub struct RealmsConnectGatingService {
    flags: Arc<FeatureFlagService>,
    analytics: Arc<AnalyticsService>,
}

impl RealmsConnectGatingService {
    pub fn new(flags: Arc<FeatureFlagService>, analytics: Arc<AnalyticsService>) -> Self {
        Self { flags, analytics }
    }

    pub fn new_shared(
        flags: Arc<FeatureFlagService>,
        analytics: Arc<AnalyticsService>,
    ) -> Arc<Self> {
        Arc::new(Self::new(flags, analytics))
    }

    pub async fn evaluate(&self, is_entitled: bool) -> RealmsGateStatus {
        let status = self.decide(is_entitled).await;
        self.record(&status);
        status
    }

    async fn decide(&self, is_entitled: bool) -> RealmsGateStatus {
        if !self.flags.get(RealmsConnectEnabled).await {
            return RealmsGateStatus::FeatureDisabled;
        }
        if self.flags.get(FreeWeekendEnabled).await {
            return RealmsGateStatus::Allowed {
                reason: GateReason::FreeWeekend,
            };
        }
        if is_entitled {
            return RealmsGateStatus::Allowed {
                reason: GateReason::Entitled,
            };
        }
        RealmsGateStatus::NotEntitled
    }

    fn record(&self, status: &RealmsGateStatus) {
        let label = match status {
            RealmsGateStatus::Allowed {
                reason: GateReason::Entitled,
            } => "allowed_entitled",
            RealmsGateStatus::Allowed {
                reason: GateReason::FreeWeekend,
            } => "allowed_free_weekend",
            RealmsGateStatus::FeatureDisabled => "feature_disabled",
            RealmsGateStatus::NotEntitled => "not_entitled",
        };
        let data = AnalyticsEventData::new().insert("result", label);
        self.analytics
            .track(AnalyticsEvent::RealmsGateEvaluated, Some(data));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn build() -> RealmsConnectGatingService {
        // Empty api_key disables Flagsmith; `get` then returns each flag's
        // conservative default (false). `initialize()` must be called or
        // `is_enabled` blocks forever waiting on the `ready` watch channel
        // (in production it is initialized at startup).
        let flags = Arc::new(FeatureFlagService::new(
            String::new(),
            String::new(),
            String::new(),
            std::time::Duration::from_secs(3600),
        ));
        flags.initialize().await;
        let telemetry = Arc::new(crate::logging::Telemetry::new(false));
        let analytics = Arc::new(AnalyticsService::new(telemetry, String::new()));
        RealmsConnectGatingService::new(flags, analytics)
    }

    #[tokio::test]
    async fn feature_disabled_when_master_flag_off() {
        let svc = build().await;
        // RealmsConnectEnabled defaults false (Flagsmith disabled) → disabled,
        // regardless of entitlement.
        assert_eq!(svc.decide(true).await, RealmsGateStatus::FeatureDisabled);
        assert_eq!(svc.decide(false).await, RealmsGateStatus::FeatureDisabled);
    }
}
