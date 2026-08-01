use bvc_client_lib::{FeatureFlagService, RealmsConnectEnabled};
use std::time::Duration;

// An empty API key is how a build without FLAGSMITH_KEY reaches this code, and
// it lands on the same path an unreachable Flagsmith does: no provider answer,
// so the flag falls back to its own default.
fn unconfigured() -> FeatureFlagService {
    FeatureFlagService::new(
        String::new(),
        String::new(),
        "install-x".to_string(),
        0,
        Duration::from_secs(3600),
        None,
    )
}

// Realms Connect costs nothing, so an outage must leave it reachable. This is
// the contract the whole fail-open design rests on, and it is only observable
// end to end: the flag's default has to survive the provider miss instead of
// being folded into `false` on the way out.
#[tokio::test]
async fn unconfigured_flagsmith_leaves_realms_connect_on() {
    let svc = unconfigured();
    svc.initialize().await;

    assert!(
        svc.get(RealmsConnectEnabled).await,
        "Realms Connect must stay enabled when Flagsmith cannot answer"
    );
}

// A read must resolve even though nothing ever publishes flags, rather than
// parking forever on the readiness signal and stalling the caller.
#[tokio::test]
async fn flag_read_resolves_without_a_provider() {
    let svc = unconfigured();
    svc.initialize().await;

    let read = tokio::time::timeout(Duration::from_secs(5), svc.get(RealmsConnectEnabled)).await;

    assert!(read.is_ok(), "flag read must not block indefinitely");
}
