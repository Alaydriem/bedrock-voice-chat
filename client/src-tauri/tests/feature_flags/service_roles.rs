use bvc_client_lib::FeatureFlagService;
use bvc_client_lib::PlatformId;
use std::time::Duration;

const DAY: i64 = 86_400;

fn svc() -> FeatureFlagService {
    // Empty api_key keeps Flagsmith disabled; we only exercise role state here.
    FeatureFlagService::new(
        String::new(),
        String::new(),
        PlatformId::new_shared("install-x".to_string()),
        0,
        Duration::from_secs(3600),
        None,
    )
}

#[test]
fn seeded_recent_roles_are_effective() {
    let s = svc();
    s.seed_discord_roles(vec!["111".into(), "222".into()], Some(1_000_000));
    let mut got = s.current_effective_roles(1_000_000 + DAY);
    got.sort();
    assert_eq!(got, vec!["111".to_string(), "222".to_string()]);
}

#[test]
fn seeded_expired_roles_drop_out() {
    let s = svc();
    s.seed_discord_roles(vec!["111".into()], Some(0));
    assert!(s.current_effective_roles(31 * DAY).is_empty());
}
