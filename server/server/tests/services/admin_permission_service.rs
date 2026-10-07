use std::sync::Arc;

use bvc_server_lib::services::{
    AdminPermissionOutcome, AdminPermissionService, PermissionService, PlayerRegistrarService,
};
use common::Game;
use common::request::admin::AdminPermissionRequest;
use common::structs::permission::{AdminAction, Permission, PermissionEffect};

use crate::harness::{CertificateFixture, DatabaseFixture};

struct Fixture {
    db: DatabaseFixture,
    registrar: PlayerRegistrarService,
    service: AdminPermissionService,
}

impl Fixture {
    async fn create() -> Self {
        let db = DatabaseFixture::create().await.expect("db fixture");
        let certs = CertificateFixture::create().expect("cert fixture");
        let conn = Arc::new(db.connection.clone());
        let registrar = PlayerRegistrarService::new(conn.clone(), certs.service.clone());
        let service = AdminPermissionService::new(conn, registrar.clone());
        Self {
            db,
            registrar,
            service,
        }
    }

    async fn apply(&self, gamertag: &str, action: AdminAction) -> AdminPermissionOutcome {
        self.service
            .apply(&AdminPermissionRequest {
                gamertag: gamertag.to_string(),
                game: Game::Minecraft,
                action,
            })
            .await
            .expect("apply")
    }

    async fn admin_override(&self, gamertag: &str) -> Option<PermissionEffect> {
        let player = self
            .registrar
            .find(gamertag, &Game::Minecraft)
            .await
            .expect("find")?;
        PermissionService::list_overrides(&self.db.connection, player.id)
            .await
            .expect("list overrides")
            .into_iter()
            .find(|(permission, _)| permission == Permission::Admin.as_str())
            .map(|(_, effect)| effect)
    }
}

#[tokio::test]
async fn grant_registers_an_unknown_gamertag_verbatim() {
    let fixture = Fixture::create().await;

    let outcome = fixture.apply("Some Name", AdminAction::Grant).await;

    assert_eq!(outcome, AdminPermissionOutcome::Applied);
    assert_eq!(
        fixture.admin_override("Some Name").await,
        Some(PermissionEffect::Allow)
    );
}

#[tokio::test]
async fn deny_replaces_a_grant() {
    let fixture = Fixture::create().await;
    fixture.apply("Alice", AdminAction::Grant).await;

    let outcome = fixture.apply("Alice", AdminAction::Deny).await;

    assert_eq!(outcome, AdminPermissionOutcome::Applied);
    assert_eq!(
        fixture.admin_override("Alice").await,
        Some(PermissionEffect::Deny)
    );
}

#[tokio::test]
async fn revoke_removes_the_override_once() {
    let fixture = Fixture::create().await;
    fixture.apply("Alice", AdminAction::Grant).await;

    let first = fixture.apply("Alice", AdminAction::Revoke).await;
    let second = fixture.apply("Alice", AdminAction::Revoke).await;

    assert_eq!(first, AdminPermissionOutcome::Applied);
    assert_eq!(second, AdminPermissionOutcome::NothingToRevoke);
    assert_eq!(fixture.admin_override("Alice").await, None);
}

#[tokio::test]
async fn deny_and_revoke_never_register_a_player() {
    let fixture = Fixture::create().await;

    let deny = fixture.apply("Mallory", AdminAction::Deny).await;
    let revoke = fixture.apply("Mallory", AdminAction::Revoke).await;

    assert_eq!(deny, AdminPermissionOutcome::PlayerNotFound);
    assert_eq!(revoke, AdminPermissionOutcome::PlayerNotFound);
    let player = fixture
        .registrar
        .find("Mallory", &Game::Minecraft)
        .await
        .expect("find");
    assert!(player.is_none());
}

#[tokio::test]
async fn a_blank_or_overlong_gamertag_is_refused_without_registering() {
    let fixture = Fixture::create().await;

    for gamertag in ["   ".to_string(), "x".repeat(65)] {
        let result = fixture
            .service
            .apply(&AdminPermissionRequest {
                gamertag: gamertag.clone(),
                game: Game::Minecraft,
                action: AdminAction::Grant,
            })
            .await;

        assert!(result.is_err(), "{gamertag:?} was accepted");
        let player = fixture
            .registrar
            .find(&gamertag, &Game::Minecraft)
            .await
            .expect("find");
        assert!(player.is_none());
    }
}
