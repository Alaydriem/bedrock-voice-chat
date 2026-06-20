pub mod realms_connect;
pub use realms_connect::RealmsConnectGatingService;

use std::sync::Arc;

use common::bedrock_protocol::ProtocolVersion;
use common::structs::{AnalyticsEvent, AnalyticsEventData};

use crate::analytics::{AnalyticsLevel, AnalyticsService};
use crate::feature_flags::FeatureFlagService;
use crate::feature_flags::flags::minecraft::{
    MaxTrustedMinecraftProtocol, MinecraftProtocolSupport,
};

// Gates inbound Minecraft client connections by negotiated protocol version.
//
// Three-layer acceptance check:
//   1. `is_supported` against `ProtocolVersion::GENERATED_ALL` (zero Flagsmith calls).
//   2. Per-version override flag `MinecraftProtocolSupport` — used to grant
//      ad-hoc access to a brand-new Minecraft release after manual
//      validation, before engineering compiles in support.
//   3. Trust-everything-up-to dial `MaxTrustedMinecraftProtocol` — single
//      setting to cover everyone when Mojang ships a no-op wire bump.
//
// Flag keys live with their `FeatureFlag` impls under
// `crate::feature_flags::flags::minecraft::*`. The service never references
// raw flag strings — all lookups go through `svc.get(<FlagStruct>)`.
pub struct ProtocolGatingService {
    flag_service: Arc<FeatureFlagService>,
    analytics: Arc<AnalyticsService>,
}

impl ProtocolGatingService {
    pub fn new(flag_service: Arc<FeatureFlagService>, analytics: Arc<AnalyticsService>) -> Self {
        Self {
            flag_service,
            analytics,
        }
    }

    pub fn new_shared(
        flag_service: Arc<FeatureFlagService>,
        analytics: Arc<AnalyticsService>,
    ) -> Arc<Self> {
        Arc::new(Self::new(flag_service, analytics))
    }

    pub fn analytics(&self) -> &Arc<AnalyticsService> {
        &self.analytics
    }

    // Whether the bundled `bedrock-protocol` codegen has emitted codecs for
    // this wire version. Anything in the upstream `GENERATED_ALL` array is
    // something the lib can decode/encode — so BVC can speak it.
    pub fn is_supported(v: ProtocolVersion) -> bool {
        ProtocolVersion::GENERATED_ALL.contains(&v)
    }

    // Decide whether to accept a connection on `protocol_version`. Emits a
    // PostHog event + Sentry breadcrumb on allow; a PostHog event + Sentry
    // warning on reject so on-call sees the spike when Mojang ships a new
    // wire version.
    pub async fn is_allowed(&self, protocol_version: ProtocolVersion) -> bool {
        let raw = protocol_version.0 as i32;

        if Self::is_supported(protocol_version) {
            return true;
        }

        if self
            .flag_service
            .get(MinecraftProtocolSupport {
                protocol_version: raw,
            })
            .await
        {
            log::info!("Minecraft protocol {raw} permitted via per-version flag");
            self.record_allowed(raw, "per_version_flag");
            return true;
        }

        if let Some(max) = self.flag_service.get(MaxTrustedMinecraftProtocol).await {
            if (raw as i64) <= max {
                log::info!("Minecraft protocol {raw} permitted via max_trusted_protocol={max}");
                self.record_allowed(raw, "max_trusted_dial");
                return true;
            }
        }

        log::warn!(
            "Minecraft protocol {raw} rejected (not in SUPPORTED_PROTOCOLS, \
             not flag-overridden)"
        );
        self.record_rejected(raw);
        false
    }

    // Build the kick message shown in the rejected client's Minecraft UI.
    // The displayed Minecraft version comes from
    // `ProtocolVersion::LATEST.client_version_str()` so the message
    // auto-tracks future LATEST bumps without code changes.
    pub fn kick_message(&self, peer_version: ProtocolVersion) -> String {
        let latest = ProtocolVersion::LATEST.client_version_str();
        format!(
            "Minecraft protocol {peer} isn't supported by BVC yet. \
             Please use Minecraft {latest}, or check for a BVC update.",
            peer = peer_version.0,
        )
    }

    fn record_allowed(&self, protocol: i32, reason: &'static str) {
        let data = AnalyticsEventData::new()
            .insert("protocol", protocol)
            .insert("reason", reason);
        self.analytics
            .track(AnalyticsEvent::MinecraftProtocolAllowed, Some(data));

        self.analytics.breadcrumb(
            "minecraft.protocol",
            &format!("protocol {protocol} allowed via {reason}"),
            AnalyticsLevel::Info,
        );
    }

    fn record_rejected(&self, protocol: i32) {
        let data = AnalyticsEventData::new().insert("protocol", protocol);
        self.analytics
            .track(AnalyticsEvent::MinecraftProtocolRejected, Some(data));

        // Warning-level capture tagged by protocol number so all rejections
        // of the same version roll into one Sentry issue with a count. A
        // spike in that count = Mojang shipped a new wire version.
        self.analytics.capture_message(
            &format!("Minecraft protocol {protocol} rejected"),
            AnalyticsLevel::Warning,
            &[("minecraft.protocol".to_string(), protocol.to_string())],
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_service() -> ProtocolGatingService {
        let flag_service = Arc::new(FeatureFlagService::new(
            String::new(),
            String::new(),
            String::new(),
            std::time::Duration::from_secs(3600),
        ));
        let telemetry = Arc::new(crate::logging::Telemetry::new(false));
        let analytics = Arc::new(AnalyticsService::new(telemetry, String::new()));
        ProtocolGatingService::new(flag_service, analytics)
    }

    #[test]
    fn kick_message_includes_peer_protocol_and_latest_version() {
        let svc = build_service();
        let msg = svc.kick_message(ProtocolVersion(988));
        let expected_latest = ProtocolVersion::LATEST.client_version_str();
        assert!(msg.contains("988"));
        assert!(msg.contains(expected_latest));
    }

    #[tokio::test]
    async fn supported_protocol_is_allowed_without_calling_flagsmith() {
        let svc = build_service();
        for v in ProtocolVersion::GENERATED_ALL {
            assert!(svc.is_allowed(v).await, "expected {v} to be allowed");
        }
    }
}
