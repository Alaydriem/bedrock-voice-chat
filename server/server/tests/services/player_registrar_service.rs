use std::sync::Arc;

use bvc_server_lib::services::PlayerRegistrarService;
use common::Game;
use entity::{player, player_identity};
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter};

use crate::harness::{CertificateFixture, DatabaseFixture, RoutingFixture};

// A gamertag the certificate cannot carry as a DNS name is still a player. Losing the
// record here leaves them in the world with no way to sign in.
#[tokio::test]
async fn a_non_ascii_gamertag_is_registered() {
    let db = DatabaseFixture::create().await.expect("db fixture");
    let certs = CertificateFixture::create().expect("cert fixture");
    let registrar =
        PlayerRegistrarService::new(Arc::new(db.connection.clone()), certs.service.clone());

    registrar
        .process_players(
            &[RoutingFixture::player("テスト ユーザー", 0.0, false)],
            Game::Minecraft,
        )
        .await;

    let found = registrar
        .find("テスト ユーザー", &Game::Minecraft)
        .await
        .expect("find");
    assert!(found.is_some());
}

// Two position ticks can reach the registrar before either has written. A new name must
// still produce one record, or every later lookup picks either one.
#[tokio::test]
async fn concurrent_first_reports_create_one_record() {
    let db = DatabaseFixture::create().await.expect("db fixture");
    let certs = CertificateFixture::create().expect("cert fixture");
    let registrar =
        PlayerRegistrarService::new(Arc::new(db.connection.clone()), certs.service.clone());
    let roster = [RoutingFixture::player("Steve", 0.0, false)];

    tokio::join!(
        registrar.process_players(&roster, Game::Minecraft),
        registrar.process_players(&roster, Game::Minecraft)
    );

    let rows = player::Entity::find()
        .filter(player::Column::Gamertag.eq("Steve"))
        .count(&db.connection)
        .await
        .expect("count");
    assert_eq!(rows, 1);
}

// An admin registers a gamertag at the moment the game server first reports it. Both go
// through one registrar, so the name still gets one record.
#[tokio::test]
async fn an_admin_create_and_a_first_report_create_one_record() {
    let db = DatabaseFixture::create().await.expect("db fixture");
    let certs = CertificateFixture::create().expect("cert fixture");
    let registrar =
        PlayerRegistrarService::new(Arc::new(db.connection.clone()), certs.service.clone());
    let roster = [RoutingFixture::player("Steve", 0.0, false)];

    let (created, _) = tokio::join!(
        registrar.create("Steve", &Game::Minecraft, None),
        registrar.process_players(&roster, Game::Minecraft)
    );

    created.expect("create");
    let rows = player::Entity::find()
        .filter(player::Column::Gamertag.eq("Steve"))
        .count(&db.connection)
        .await
        .expect("count");
    assert_eq!(rows, 1);
}

#[tokio::test]
async fn creating_an_existing_player_returns_it_without_a_second_row() {
    let db = DatabaseFixture::create().await.expect("db fixture");
    let certs = CertificateFixture::create().expect("cert fixture");
    let registrar =
        PlayerRegistrarService::new(Arc::new(db.connection.clone()), certs.service.clone());

    let first = registrar
        .create("Alice", &Game::Minecraft, None)
        .await
        .expect("first create");
    let second = registrar
        .create("Alice", &Game::Minecraft, None)
        .await
        .expect("second create");

    assert_eq!(first.id, second.id);
    let rows = player::Entity::find()
        .filter(player::Column::Gamertag.eq("Alice"))
        .count(&db.connection)
        .await
        .expect("count");
    assert_eq!(rows, 1);
}

#[tokio::test]
async fn a_uuid_supplied_for_an_existing_player_is_recorded() {
    let db = DatabaseFixture::create().await.expect("db fixture");
    let certs = CertificateFixture::create().expect("cert fixture");
    let registrar =
        PlayerRegistrarService::new(Arc::new(db.connection.clone()), certs.service.clone());

    let existing = registrar
        .create("Carol", &Game::Minecraft, None)
        .await
        .expect("first create");
    registrar
        .create("Carol", &Game::Minecraft, Some("0b7f3c1e-uuid"))
        .await
        .expect("second create");

    let identity = player_identity::Entity::find()
        .filter(player_identity::Column::PlayerId.eq(existing.id))
        .filter(player_identity::Column::Alias.eq("0b7f3c1e-uuid"))
        .one(&db.connection)
        .await
        .expect("query");
    assert!(identity.is_some());
}

#[tokio::test]
async fn find_matches_the_gamertag_case_sensitively() {
    let db = DatabaseFixture::create().await.expect("db fixture");
    let certs = CertificateFixture::create().expect("cert fixture");
    let registrar =
        PlayerRegistrarService::new(Arc::new(db.connection.clone()), certs.service.clone());

    registrar
        .create("Alice", &Game::Minecraft, None)
        .await
        .expect("create");

    let exact = registrar.find("Alice", &Game::Minecraft).await.expect("find");
    let other_case = registrar.find("alice", &Game::Minecraft).await.expect("find");

    assert!(exact.is_some());
    assert!(other_case.is_none());
}
