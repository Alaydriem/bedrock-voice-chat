use bvc_client_lib::DiscordLinkService;

const DAY: i64 = 86_400;

#[test]
fn status_linked_and_fresh() {
    let st = DiscordLinkService::build_status(
        &["111".into(), "222".into()],
        Some(1_000_000),
        1_000_000 + DAY,
        true,
    );
    assert!(st.configured);
    assert!(st.linked);
    assert_eq!(st.role_count, 2);
    assert_eq!(st.last_synced, Some(1_000_000));
    assert!(!st.expired);
}

#[test]
fn status_linked_but_expired() {
    let st = DiscordLinkService::build_status(&["111".into()], Some(0), 31 * DAY, true);
    assert!(st.linked);
    assert!(st.expired);
}

#[test]
fn status_never_linked() {
    let st = DiscordLinkService::build_status(&[], None, 1_000_000, true);
    assert!(!st.linked);
    assert!(st.expired);
    assert_eq!(st.role_count, 0);
}

#[test]
fn status_not_configured() {
    let st = DiscordLinkService::build_status(&[], None, 1_000_000, false);
    assert!(!st.configured);
}
