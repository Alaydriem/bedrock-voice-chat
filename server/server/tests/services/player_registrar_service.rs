//! The registrar's create is idempotent: callers that cannot know whether a player exists
//! (the FFI admin and provisioning exports) rely on it never inserting a second row.

use std::sync::Arc;

use bvc_server_lib::services::PlayerRegistrarService;
use common::Game;
use entity::{player, player_identity};
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter};

use crate::harness::{CertificateFixture, DatabaseFixture};

#[tokio::test]
async fn creating_an_existing_player_returns_it_without_a_second_row() {
    let db = DatabaseFixture::create().await.expect("db fixture");
    let certs = CertificateFixture::create().expect("cert fixture");
    let registrar =
        PlayerRegistrarService::new(Arc::new(db.connection.clone()), certs.service.clone());

    let first = registrar
        .create("Alice", Some(&Game::Minecraft), None)
        .await
        .expect("first create");
    let second = registrar
        .create("Alice", Some(&Game::Minecraft), None)
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
async fn no_game_creates_a_minecraft_player() {
    let db = DatabaseFixture::create().await.expect("db fixture");
    let certs = CertificateFixture::create().expect("cert fixture");
    let registrar =
        PlayerRegistrarService::new(Arc::new(db.connection.clone()), certs.service.clone());

    let created = registrar.create("Bob", None, None).await.expect("create");

    assert_eq!(created.game, Game::Minecraft);
}

#[tokio::test]
async fn a_uuid_supplied_for_an_existing_player_is_recorded() {
    let db = DatabaseFixture::create().await.expect("db fixture");
    let certs = CertificateFixture::create().expect("cert fixture");
    let registrar =
        PlayerRegistrarService::new(Arc::new(db.connection.clone()), certs.service.clone());

    let existing = registrar
        .create("Carol", None, None)
        .await
        .expect("first create");
    registrar
        .create("Carol", None, Some("0b7f3c1e-uuid"))
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
