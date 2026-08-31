use bvc_relay_service::discord::{DiscordBotClient, FixedMemberSource, MemberSource};

// A member who has left the guild has no roles rather than an error. Departure and
// cancellation are the same observation at this API, and both mean "not entitled" —
// but neither is an outage, and treating them as one would suspend names whenever
// Discord returned a 404.
#[tokio::test]
async fn a_member_absent_from_the_guild_has_no_roles() {
    let source = MemberSource::Fixed(FixedMemberSource::absent());

    let roles = source
        .role_ids("member-1")
        .await
        .expect("absence is not an error");

    assert!(roles.is_empty());
}

#[tokio::test]
async fn a_member_present_in_the_guild_reports_their_roles() {
    let source = MemberSource::Fixed(FixedMemberSource::new(vec!["role-a".to_string()]));

    let roles = source.role_ids("member-1").await.expect("present");

    assert_eq!(roles, vec!["role-a".to_string()]);
}

// The member payload's `roles` array is the only field read. A response missing it
// is a member with no roles, not a parse failure.
#[test]
fn a_member_payload_without_roles_yields_none() {
    let body = serde_json::json!({ "user": { "id": "member-1" } });

    assert!(DiscordBotClient::roles_from_member(&body).is_empty());
}

#[test]
fn a_member_payload_with_roles_yields_them() {
    let body = serde_json::json!({ "roles": ["role-a", "role-b"] });

    assert_eq!(
        DiscordBotClient::roles_from_member(&body),
        vec!["role-a".to_string(), "role-b".to_string()]
    );
}
