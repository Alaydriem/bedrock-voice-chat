use std::sync::Arc;

use bvc_relay_service::config::DiscordConfig;
use bvc_relay_service::db::Db;
use bvc_relay_service::discord::{FixedMemberSource, MemberSource};
use bvc_relay_service::registry::RegistryService;

fn discord_config() -> DiscordConfig {
    DiscordConfig {
        guild_id: "guild".to_string(),
        bot_token: "bot".to_string(),
        client_id: "client".to_string(),
        client_secret: "secret".to_string(),
        qualifying_role_ids: vec!["role-a".to_string()],
    }
}

async fn service(roles: Vec<String>) -> Arc<RegistryService> {
    let conn = Arc::new(Db::connect("sqlite::memory:").await.expect("connects"));
    RegistryService::new_shared(
        conn,
        discord_config(),
        MemberSource::Fixed(FixedMemberSource::new(roles)),
    )
}

#[tokio::test]
async fn an_entitled_member_is_issued_a_token() {
    let service = service(vec!["role-a".to_string()]).await;

    let token = service.issue_token("member-1").await.expect("issues");

    assert!(token.starts_with("bvcenroll"));
}

#[tokio::test]
async fn an_unentitled_member_is_refused_a_token() {
    let service = service(vec!["role-z".to_string()]).await;

    assert!(service.issue_token("member-1").await.is_err());
}

// Redemption binds the token's member to the connection's node id and returns the
// assigned name.
#[tokio::test]
async fn redeeming_a_token_assigns_a_name() {
    let service = service(vec!["role-a".to_string()]).await;
    let token = service.issue_token("member-1").await.expect("issues");

    let name = service.redeem(&token, "node-a").await.expect("redeems");

    assert!(!name.is_empty());
    assert_eq!(service.name_for("node-a").await.expect("lookup"), Some(name));
}

// Single use. A leaked config file after first boot grants nothing.
#[tokio::test]
async fn a_token_cannot_be_redeemed_twice() {
    let service = service(vec!["role-a".to_string()]).await;
    let token = service.issue_token("member-1").await.expect("issues");
    service
        .redeem(&token, "node-a")
        .await
        .expect("first redemption");

    assert!(service.redeem(&token, "node-b").await.is_err());
}

// One name per member. The schema enforces it too; this is the error the operator
// actually sees.
#[tokio::test]
async fn a_member_with_a_live_registration_is_refused_a_second_token() {
    let service = service(vec!["role-a".to_string()]).await;
    let token = service.issue_token("member-1").await.expect("issues");
    service.redeem(&token, "node-a").await.expect("redeems");

    assert!(service.issue_token("member-1").await.is_err());
}

// Suspension withdraws the registration without retiring the name, so recovery is a
// state change rather than a reassignment.
#[tokio::test]
async fn a_suspended_registration_no_longer_resolves_to_its_name() {
    let service = service(vec!["role-a".to_string()]).await;
    let token = service.issue_token("member-1").await.expect("issues");
    service.redeem(&token, "node-a").await.expect("redeems");

    service.suspend("node-a").await.expect("suspends");

    assert_eq!(service.name_for("node-a").await.expect("lookup"), None);
}

// A membership that lapses between issuance and redemption is caught at redemption.
// The token is valid for a day, which is long enough for one to expire inside its
// own window.
#[tokio::test]
async fn a_membership_that_lapses_before_redemption_is_refused() {
    let conn = Arc::new(Db::connect("sqlite::memory:").await.expect("connects"));
    let entitled = RegistryService::new_shared(
        conn.clone(),
        discord_config(),
        MemberSource::Fixed(FixedMemberSource::new(vec!["role-a".to_string()])),
    );
    let token = entitled.issue_token("member-1").await.expect("issues");

    let lapsed = RegistryService::new_shared(
        conn,
        discord_config(),
        MemberSource::Fixed(FixedMemberSource::absent()),
    );

    assert!(lapsed.redeem(&token, "node-a").await.is_err());
}

// A declared address is recorded, not just published. The daily pass reads this
// column to decide whether to bind the record to the node, so an address published
// without being recorded is one nothing ever verifies.
#[tokio::test]
async fn declaring_an_address_records_it_against_the_registration() {
    let service = service(vec!["role-a".to_string()]).await;
    let token = service.issue_token("member-1").await.expect("issues");
    let name = service.redeem(&token, "node-a").await.expect("redeems");

    let declared = service
        .declare_address("node-a", "203.0.113.10")
        .await
        .expect("declares");

    assert_eq!(declared, name);
    assert_eq!(
        service.declared_address("node-a").await.expect("lookup"),
        Some("203.0.113.10".to_string())
    );
}

// A node with no registration cannot publish an address record into the zone.
#[tokio::test]
async fn an_unregistered_node_cannot_declare_an_address() {
    let service = service(vec!["role-a".to_string()]).await;

    assert!(service.declare_address("node-a", "203.0.113.10").await.is_err());
}

// A suspended registration cannot re-publish its address. Suspension withdraws the
// record, and accepting a declaration would put it straight back.
#[tokio::test]
async fn a_suspended_registration_cannot_declare_an_address() {
    let service = service(vec!["role-a".to_string()]).await;
    let token = service.issue_token("member-1").await.expect("issues");
    service.redeem(&token, "node-a").await.expect("redeems");
    service.suspend("node-a").await.expect("suspends");

    assert!(service.declare_address("node-a", "203.0.113.10").await.is_err());
}
