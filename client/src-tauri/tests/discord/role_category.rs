use bvc_client_lib::RoleCategory;

fn labels(ids: &[&str]) -> Vec<String> {
    let owned: Vec<String> = ids.iter().map(|s| s.to_string()).collect();
    let mut got = RoleCategory::labels_for(&owned);
    got.sort();
    got
}

#[test]
fn single_tier_role_yields_label_plus_umbrella() {
    assert_eq!(
        labels(&["1447055535294906440"]),
        vec!["sponsor".to_string(), "supporter-any".to_string()]
    );
}

#[test]
fn second_source_role_in_a_tier_also_matches() {
    // The YouTube/Patreon-synced sibling ID of the same tier resolves to the
    // same label.
    assert_eq!(
        labels(&["1447080496214315131"]),
        vec!["supporter".to_string(), "supporter-any".to_string()]
    );
}

#[test]
fn gifted_access_matches() {
    assert_eq!(
        labels(&["1519551186934562917"]),
        vec!["gifted-bvc-access".to_string(), "supporter-any".to_string()]
    );
}

#[test]
fn multiple_tiers_yield_all_labels_and_one_umbrella() {
    assert_eq!(
        labels(&["1519548906642346054", "1447055535294906440"]),
        vec![
            "foundational-supporter".to_string(),
            "sponsor".to_string(),
            "supporter-any".to_string(),
        ]
    );
}

#[test]
fn unmatched_roles_yield_no_labels() {
    assert!(RoleCategory::labels_for(&["999".to_string()]).is_empty());
    assert!(RoleCategory::labels_for(&[]).is_empty());
}
