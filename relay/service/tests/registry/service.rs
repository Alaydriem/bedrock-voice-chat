use std::sync::Arc;

use bvc_relay_service::config::DiscordConfig;
use bvc_relay_service::db::Db;
use bvc_relay_service::discord::{FixedMemberSource, MemberSource};
use bvc_relay_service::entity::registration;
use bvc_relay_service::registry::{RegistryError, RegistryService};
use sea_orm::{ConnectionTrait, DatabaseConnection, EntityTrait};

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
    assert_eq!(
        service.name_for("node-a").await.expect("lookup"),
        Some(name)
    );
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

async fn registered_service() -> (Arc<RegistryService>, Arc<DatabaseConnection>, String) {
    let conn = Arc::new(Db::connect("sqlite::memory:").await.expect("connects"));
    let service = RegistryService::new_shared(
        conn.clone(),
        discord_config(),
        MemberSource::Fixed(FixedMemberSource::new(vec!["role-a".to_string()])),
    );
    let token = service.issue_token("member-1").await.expect("issues");
    let name = service.redeem(&token, "node-a").await.expect("redeems");
    (service, conn, name)
}

async fn row(conn: &DatabaseConnection) -> registration::Model {
    registration::Entity::find_by_id("node-a")
        .one(conn)
        .await
        .expect("queries")
        .expect("row exists")
}

#[tokio::test]
async fn a_check_in_records_address_port_and_nonce() {
    let (service, conn, _) = registered_service().await;

    let record = service
        .record_check_in("node-a", Some("203.0.113.5".parse().unwrap()), 8443)
        .await
        .expect("checks in");

    let row = row(&conn).await;
    assert_eq!(row.declared_address.as_deref(), Some("203.0.113.5"));
    assert_eq!(row.declared_port, Some(8443));
    assert!(!record.nonce.is_empty());
    assert_eq!(row.checkin_nonce.as_deref(), Some(record.nonce.as_str()));
    assert!(row.checked_in_at.is_some());
}

#[tokio::test]
async fn a_check_in_with_a_new_port_clears_verification() {
    let (service, conn, _) = registered_service().await;
    let address: std::net::IpAddr = "203.0.113.5".parse().unwrap();
    service
        .record_check_in("node-a", Some(address), 8443)
        .await
        .expect("first check-in");
    conn.execute_unprepared("UPDATE registration SET address_verified_at = 100")
        .await
        .expect("marks verified");

    let record = service
        .record_check_in("node-a", Some(address), 9443)
        .await
        .expect("second check-in");

    assert!(record.address_changed);
    assert_eq!(row(&conn).await.address_verified_at, None);
}

#[tokio::test]
async fn a_suspended_registration_cannot_check_in() {
    let (service, conn, _) = registered_service().await;
    conn.execute_unprepared("UPDATE registration SET state = 'suspended'")
        .await
        .expect("suspends");
    let before = row(&conn).await;

    let result = service
        .record_check_in("node-a", Some("203.0.113.5".parse().unwrap()), 8443)
        .await;

    assert!(matches!(result, Err(RegistryError::Suspended)));
    assert_eq!(row(&conn).await, before);
}

#[tokio::test]
async fn a_revoked_entitled_registration_may_publish_challenges() {
    let (service, _, name) = registered_service().await;

    service.revoke("node-a").await.expect("revokes");

    assert_eq!(service.name_for("node-a").await.expect("lookup"), Some(name));
}

#[tokio::test]
async fn a_revoked_unentitled_registration_may_not_publish_challenges() {
    let (service, _, _) = registered_service().await;
    service.revoke("node-a").await.expect("revokes");
    service
        .record_entitlement_check("node-a", Some(false))
        .await
        .expect("records");

    assert_eq!(service.name_for("node-a").await.expect("lookup"), None);
}

#[tokio::test]
async fn a_recovery_evaluation_is_claimed_once_per_window() {
    let (service, conn, _) = registered_service().await;
    conn.execute_unprepared("UPDATE registration SET entitlement_checked_at = NULL")
        .await
        .expect("clears");

    let first = service
        .claim_recovery_evaluation("node-a", 600)
        .await
        .expect("claims");
    let second = service
        .claim_recovery_evaluation("node-a", 600)
        .await
        .expect("claims");

    assert!(first);
    assert!(!second);
    assert!(row(&conn).await.entitlement_checked_at.is_some());
}

#[tokio::test]
async fn a_recovery_evaluation_cannot_be_claimed_for_an_unknown_node() {
    let (service, _, _) = registered_service().await;

    assert!(
        !service
            .claim_recovery_evaluation("node-z", 600)
            .await
            .expect("claims")
    );
}

#[tokio::test]
async fn reinstating_restores_the_name() {
    let (service, conn, name) = registered_service().await;
    service.revoke("node-a").await.expect("revokes");
    assert!(row(&conn).await.revoked_at.is_some());

    service.reinstate("node-a").await.expect("reinstates");

    let row = row(&conn).await;
    assert_eq!(row.state, "active");
    assert_eq!(row.revoked_at, None);
    assert_eq!(service.name_for("node-a").await.expect("lookup"), Some(name));
}
