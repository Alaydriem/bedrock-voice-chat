use bvc_client_lib::DiscordTraitState;

const DAY: i64 = 86_400;

#[test]
fn within_window_returns_roles() {
    let s = DiscordTraitState {
        roles: vec!["111".into(), "222".into()],
        last_sync: Some(1_000_000),
    };
    assert_eq!(
        s.effective_roles(1_000_000 + 10 * DAY),
        vec!["111".to_string(), "222".to_string()]
    );
    assert!(!s.is_expired(1_000_000 + 10 * DAY));
}

#[test]
fn never_synced_returns_empty_and_is_expired() {
    let s = DiscordTraitState { roles: vec!["111".into()], last_sync: None };
    assert!(s.effective_roles(1_000_000).is_empty());
    assert!(s.is_expired(1_000_000));
}

#[test]
fn exactly_30_days_is_still_valid() {
    let s = DiscordTraitState { roles: vec!["x".into()], last_sync: Some(0) };
    assert_eq!(s.effective_roles(30 * DAY), vec!["x".to_string()]);
    assert!(!s.is_expired(30 * DAY));
}

#[test]
fn past_30_days_returns_empty_and_is_expired() {
    let s = DiscordTraitState { roles: vec!["x".into()], last_sync: Some(0) };
    assert!(s.effective_roles(30 * DAY + 1).is_empty());
    assert!(s.is_expired(30 * DAY + 1));
}
