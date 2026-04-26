mod support;

fn minecraft_position_body(
    name: &str,
    player_uuid: Option<&str>,
    alternative_identity: Option<&str>,
) -> serde_json::Value {
    serde_json::json!({
        "game": "minecraft",
        "players": [{
            "name": name,
            "coordinates": { "x": 0.0, "y": 64.0, "z": 0.0 },
            "orientation": { "x": 0.0, "y": 0.0 },
            "dimension": "overworld",
            "deafen": false,
            "spectator": false,
            "world_uuid": "world-1",
            "alternative_identity": alternative_identity,
            "player_uuid": player_uuid
        }]
    })
}

#[tokio::test]
async fn a1_java_first_auto_register_writes_uuid_alias() {
    use common::Game;
    use entity::{player, player_identity};
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

    let db = support::fresh_in_memory_db().await;
    let fake = std::sync::Arc::new(support::FakeMinecraftAuthenticator::new(Err(
        common::auth::AuthError::AuthenticationFailed("unused".into()),
    )));
    let client = support::build_test_client(db.clone(), fake).await;

    let body = minecraft_position_body("CoolBuilder42", Some("java-uuid-1"), None);
    let resp = client
        .post("/api/position")
        .header(rocket::http::Header::new("X-MC-Access-Token", "test-bypass-token"))
        .json(&body)
        .dispatch()
        .await;
    assert_eq!(resp.status(), rocket::http::Status::Ok);

    let players: Vec<player::Model> = player::Entity::find()
        .filter(player::Column::Game.eq(Game::Minecraft))
        .all(db.as_ref())
        .await
        .unwrap();
    assert_eq!(players.len(), 1);
    assert_eq!(players[0].gamertag.as_deref(), Some("CoolBuilder42"));

    let aliases: Vec<player_identity::Model> = player_identity::Entity::find()
        .filter(player_identity::Column::PlayerId.eq(players[0].id))
        .all(db.as_ref())
        .await
        .unwrap();
    assert!(
        aliases
            .iter()
            .any(|a| a.alias == "java-uuid-1" && a.alias_type == "platform_uuid"),
        "expected platform_uuid alias for java-uuid-1"
    );
}

#[tokio::test]
async fn a2_uuid_alias_refreshes_minecraft_services_alias_on_rename() {
    use common::Game;
    use entity::{player, player_identity};
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

    let db = support::fresh_in_memory_db().await;
    let fake = std::sync::Arc::new(support::FakeMinecraftAuthenticator::new(Err(
        common::auth::AuthError::AuthenticationFailed("unused".into()),
    )));
    let client = support::build_test_client(db.clone(), fake).await;

    let xbl_id = support::seed_player(&db.conn, "AwesomeXboxer123", &Game::Minecraft).await;
    support::seed_alias(&db.conn, xbl_id, "java-uuid-1", "platform_uuid", &Game::Minecraft).await;
    support::seed_alias(&db.conn, xbl_id, "OldName", "minecraft_services", &Game::Minecraft).await;

    let body = minecraft_position_body("NewName", Some("java-uuid-1"), None);
    let resp = client
        .post("/api/position")
        .header(rocket::http::Header::new("X-MC-Access-Token", "test-bypass-token"))
        .json(&body)
        .dispatch()
        .await;
    assert_eq!(resp.status(), rocket::http::Status::Ok);

    let players: Vec<player::Model> = player::Entity::find()
        .filter(player::Column::Game.eq(Game::Minecraft))
        .all(db.as_ref())
        .await
        .unwrap();
    assert_eq!(
        players.len(),
        1,
        "no new player should be created (UUID resolves to existing XBL record)"
    );

    let aliases: Vec<player_identity::Model> = player_identity::Entity::find()
        .filter(player_identity::Column::PlayerId.eq(xbl_id))
        .all(db.as_ref())
        .await
        .unwrap();
    let names: std::collections::HashSet<_> = aliases
        .iter()
        .filter(|a| a.alias_type == "minecraft_services")
        .map(|a| a.alias.clone())
        .collect();
    assert!(
        names.contains("NewName"),
        "new java name should be aliased after rename; found: {:?}",
        names
    );
}

#[tokio::test]
async fn a3_uuid_alias_remaps_to_xbl_record() {
    use common::Game;
    use entity::player;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

    let db = support::fresh_in_memory_db().await;
    let fake = std::sync::Arc::new(support::FakeMinecraftAuthenticator::new(Err(
        common::auth::AuthError::AuthenticationFailed("unused".into()),
    )));
    let client = support::build_test_client(db.clone(), fake).await;

    let xbl_id = support::seed_player(&db.conn, "AwesomeXboxer123", &Game::Minecraft).await;
    support::seed_alias(&db.conn, xbl_id, "java-uuid-1", "platform_uuid", &Game::Minecraft).await;

    let body = minecraft_position_body("CoolBuilder42", Some("java-uuid-1"), None);
    let resp = client
        .post("/api/position")
        .header(rocket::http::Header::new("X-MC-Access-Token", "test-bypass-token"))
        .json(&body)
        .dispatch()
        .await;
    assert_eq!(resp.status(), rocket::http::Status::Ok);

    let players: Vec<player::Model> = player::Entity::find()
        .filter(player::Column::Game.eq(Game::Minecraft))
        .all(db.as_ref())
        .await
        .unwrap();
    assert_eq!(players.len(), 1, "no new player should be created");
    assert_eq!(
        players[0].gamertag.as_deref(),
        Some("AwesomeXboxer123"),
        "player should be the XBL record"
    );
}

#[tokio::test]
async fn a4_floodgate_alternative_identity_still_creates_alias() {
    use common::Game;
    use entity::player_identity;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

    let db = support::fresh_in_memory_db().await;
    let fake = std::sync::Arc::new(support::FakeMinecraftAuthenticator::new(Err(
        common::auth::AuthError::AuthenticationFailed("unused".into()),
    )));
    let client = support::build_test_client(db.clone(), fake).await;

    let xbl_id = support::seed_player(&db.conn, "BedrockTag", &Game::Minecraft).await;

    let body = minecraft_position_body(".BedrockTag", None, Some("BedrockTag"));
    let resp = client
        .post("/api/position")
        .header(rocket::http::Header::new("X-MC-Access-Token", "test-bypass-token"))
        .json(&body)
        .dispatch()
        .await;
    assert_eq!(resp.status(), rocket::http::Status::Ok);

    let aliases: Vec<player_identity::Model> = player_identity::Entity::find()
        .filter(player_identity::Column::PlayerId.eq(xbl_id))
        .all(db.as_ref())
        .await
        .unwrap();
    assert!(
        aliases
            .iter()
            .any(|a| a.alias == ".BedrockTag" && a.alias_type == "floodgate"),
        "expected floodgate alias for .BedrockTag; found: {:?}",
        aliases.iter().map(|a| (&a.alias, &a.alias_type)).collect::<Vec<_>>()
    );
}

#[tokio::test]
async fn a5_uuid_shadows_orphan_gamertag_lookup() {
    use common::Game;
    use entity::player;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

    let db = support::fresh_in_memory_db().await;
    let fake = std::sync::Arc::new(support::FakeMinecraftAuthenticator::new(Err(
        common::auth::AuthError::AuthenticationFailed("unused".into()),
    )));
    let client = support::build_test_client(db.clone(), fake).await;

    let orphan_id = support::seed_player(&db.conn, "CoolBuilder42", &Game::Minecraft).await;
    let xbl_id = support::seed_player(&db.conn, "AwesomeXboxer123", &Game::Minecraft).await;
    support::seed_alias(&db.conn, xbl_id, "java-uuid-1", "platform_uuid", &Game::Minecraft).await;

    let body = minecraft_position_body("CoolBuilder42", Some("java-uuid-1"), None);
    let resp = client
        .post("/api/position")
        .header(rocket::http::Header::new("X-MC-Access-Token", "test-bypass-token"))
        .json(&body)
        .dispatch()
        .await;
    assert_eq!(resp.status(), rocket::http::Status::Ok);

    let players: Vec<player::Model> = player::Entity::find()
        .filter(player::Column::Game.eq(Game::Minecraft))
        .all(db.as_ref())
        .await
        .unwrap();
    assert_eq!(
        players.len(),
        2,
        "orphan + XBL records, no third created; found: {:?}",
        players.iter().map(|p| (&p.id, &p.gamertag)).collect::<Vec<_>>()
    );
    let ids: std::collections::HashSet<i32> = players.iter().map(|p| p.id).collect();
    assert!(ids.contains(&orphan_id), "orphan record should still exist");
    assert!(ids.contains(&xbl_id), "XBL record should still exist");
}

#[tokio::test]
async fn migrations_apply_against_in_memory_db() {
    let db = support::fresh_in_memory_db().await;
    let pong = sea_orm::ConnectionTrait::execute_unprepared(db.as_ref(), "SELECT 1").await;
    assert!(pong.is_ok());
}

#[tokio::test]
async fn fake_authenticator_returns_canned_result() {
    use common::auth::{AuthResult, MinecraftAuthenticator};
    let fake = support::FakeMinecraftAuthenticator::new(Ok(
        AuthResult::new("Tag".into(), "pic".into())
            .with_java_profile(Some("Java".into()), Some("uuid-1".into()))
    ));
    let r = fake.authenticate("code".into(), "http://x/cb".parse().unwrap()).await.unwrap();
    assert_eq!(r.gamertag, "Tag");
    assert_eq!(r.minecraft_username.as_deref(), Some("Java"));
    assert_eq!(r.minecraft_uuid.as_deref(), Some("uuid-1"));
}

#[tokio::test]
async fn test_client_builds_and_responds() {
    use common::auth::AuthResult;
    let db = support::fresh_in_memory_db().await;
    let fake = std::sync::Arc::new(support::FakeMinecraftAuthenticator::new(Ok(
        AuthResult::new("AwesomeXboxer123".into(), "pic".into())
            .with_java_profile(Some("CoolBuilder42".into()), Some("uuid-1".into()))
    )));
    let client = support::build_test_client(db, fake).await;

    let body = serde_json::json!({
        "code": "auth-code",
        "redirect_uri": "http://app/cb"
    });
    let response = client.post("/api/auth/minecraft").json(&body).dispatch().await;
    let _ = response.status();
}

#[tokio::test]
async fn b1_xbl_first_creates_both_aliases() {
    use common::auth::AuthResult;
    use common::Game;
    use entity::{player, player_identity};
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

    let db = support::fresh_in_memory_db().await;
    support::seed_player(&db.conn, "AwesomeXboxer123", &Game::Minecraft).await;
    let fake = std::sync::Arc::new(support::FakeMinecraftAuthenticator::new(Ok(
        AuthResult::new("AwesomeXboxer123".into(), "pic".into())
            .with_java_profile(Some("CoolBuilder42".into()), Some("java-uuid-1".into()))
    )));
    let client = support::build_test_client(db.clone(), fake).await;

    let resp = client.post("/api/auth/minecraft")
        .json(&serde_json::json!({ "code": "c", "redirect_uri": "http://app/cb" }))
        .dispatch().await;
    assert_eq!(resp.status(), rocket::http::Status::Ok, "login should succeed");

    let players: Vec<player::Model> = player::Entity::find()
        .filter(player::Column::Game.eq(Game::Minecraft))
        .all(db.as_ref()).await.unwrap();
    assert_eq!(players.len(), 1);
    assert_eq!(players[0].gamertag.as_deref(), Some("AwesomeXboxer123"));

    let aliases: Vec<player_identity::Model> = player_identity::Entity::find()
        .filter(player_identity::Column::PlayerId.eq(players[0].id))
        .all(db.as_ref()).await.unwrap();
    let kinds: std::collections::HashSet<_> = aliases.iter()
        .map(|a| (a.alias.clone(), a.alias_type.clone())).collect();
    assert!(
        kinds.contains(&("CoolBuilder42".to_string(), "minecraft_services".to_string())),
        "expected minecraft_services alias for CoolBuilder42"
    );
    assert!(
        kinds.contains(&("java-uuid-1".to_string(), "platform_uuid".to_string())),
        "expected platform_uuid alias for java-uuid-1"
    );
}

#[tokio::test]
async fn b2_same_gamertag_and_java_name_skips_name_alias() {
    use common::auth::AuthResult;
    use common::Game;
    use entity::player_identity;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

    let db = support::fresh_in_memory_db().await;
    support::seed_player(&db.conn, "Same", &Game::Minecraft).await;
    let fake = std::sync::Arc::new(support::FakeMinecraftAuthenticator::new(Ok(
        AuthResult::new("Same".into(), "pic".into())
            .with_java_profile(Some("Same".into()), Some("uuid-x".into()))
    )));
    let client = support::build_test_client(db.clone(), fake).await;

    let resp = client.post("/api/auth/minecraft")
        .json(&serde_json::json!({ "code": "c", "redirect_uri": "http://app/cb" }))
        .dispatch().await;
    assert_eq!(resp.status(), rocket::http::Status::Ok);

    let aliases: Vec<player_identity::Model> = player_identity::Entity::find()
        .filter(player_identity::Column::Game.eq(Game::Minecraft))
        .all(db.as_ref()).await.unwrap();

    let types: std::collections::HashSet<_> = aliases.iter().map(|a| a.alias_type.clone()).collect();
    assert!(types.contains("platform_uuid"), "platform_uuid should be present");
    assert!(!types.contains("minecraft_services"), "minecraft_services should be SKIPPED when java==gamertag");
}

#[tokio::test]
async fn b3_bedrock_only_account_writes_no_aliases() {
    use common::auth::AuthResult;
    use common::Game;
    use entity::{player, player_identity};
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

    let db = support::fresh_in_memory_db().await;
    support::seed_player(&db.conn, "BedrockGuy", &Game::Minecraft).await;
    let fake = std::sync::Arc::new(support::FakeMinecraftAuthenticator::new(Ok(
        AuthResult::new("BedrockGuy".into(), "pic".into())
            .with_java_profile(None, None)
    )));
    let client = support::build_test_client(db.clone(), fake).await;

    let resp = client.post("/api/auth/minecraft")
        .json(&serde_json::json!({ "code": "c", "redirect_uri": "http://app/cb" }))
        .dispatch().await;
    assert_eq!(resp.status(), rocket::http::Status::Ok);

    let players: Vec<player::Model> = player::Entity::find()
        .filter(player::Column::Game.eq(Game::Minecraft))
        .all(db.as_ref()).await.unwrap();
    assert_eq!(players.len(), 1);

    let aliases: Vec<player_identity::Model> = player_identity::Entity::find()
        .filter(player_identity::Column::Game.eq(Game::Minecraft))
        .all(db.as_ref()).await.unwrap();
    assert_eq!(aliases.len(), 0);
}

#[tokio::test]
async fn b4_java_first_then_xbl_login_creates_fresh_xbl_record_and_repoints_aliases() {
    use common::auth::AuthResult;
    use common::Game;
    use entity::{player, player_identity};
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

    let db = support::fresh_in_memory_db().await;

    let java_id = support::seed_player(&db.conn, "CoolBuilder42", &Game::Minecraft).await;
    support::seed_alias(&db.conn, java_id, "java-uuid-1", "platform_uuid", &Game::Minecraft).await;

    let fake = std::sync::Arc::new(support::FakeMinecraftAuthenticator::new(Ok(
        AuthResult::new("AwesomeXboxer123".into(), "pic".into())
            .with_java_profile(Some("CoolBuilder42".into()), Some("java-uuid-1".into()))
    )));
    let client = support::build_test_client(db.clone(), fake).await;

    let resp = client.post("/api/auth/minecraft")
        .json(&serde_json::json!({ "code": "c", "redirect_uri": "http://app/cb" }))
        .dispatch().await;
    assert_eq!(resp.status(), rocket::http::Status::Ok);

    let players: Vec<player::Model> = player::Entity::find()
        .filter(player::Column::Game.eq(Game::Minecraft))
        .all(db.as_ref()).await.unwrap();
    let xbl_record = players.iter()
        .find(|p| p.gamertag.as_deref() == Some("AwesomeXboxer123"))
        .expect("new XBL record must exist");
    assert!(players.iter().any(|p| p.id == java_id),
        "orphan Java-first record should remain");

    let uuid_alias: player_identity::Model = player_identity::Entity::find()
        .filter(player_identity::Column::Alias.eq("java-uuid-1"))
        .filter(player_identity::Column::AliasType.eq("platform_uuid"))
        .one(db.as_ref()).await.unwrap()
        .expect("uuid alias");
    assert_eq!(uuid_alias.player_id, xbl_record.id,
        "UUID alias must re-point to the new XBL record");

    let name_alias: player_identity::Model = player_identity::Entity::find()
        .filter(player_identity::Column::Alias.eq("CoolBuilder42"))
        .filter(player_identity::Column::AliasType.eq("minecraft_services"))
        .one(db.as_ref()).await.unwrap()
        .expect("name alias");
    assert_eq!(name_alias.player_id, xbl_record.id,
        "Java name alias must point to the new XBL record");
}

#[tokio::test]
async fn b5_operator_whitelist_race_repoints_aliases_to_xbl() {
    use common::auth::AuthResult;
    use common::Game;
    use entity::player_identity;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

    let db = support::fresh_in_memory_db().await;
    let xbl_id = support::seed_player(&db.conn, "AwesomeXboxer123", &Game::Minecraft).await;
    let java_id = support::seed_player(&db.conn, "CoolBuilder42", &Game::Minecraft).await;
    support::seed_alias(&db.conn, java_id, "java-uuid-1", "platform_uuid", &Game::Minecraft).await;

    let fake = std::sync::Arc::new(support::FakeMinecraftAuthenticator::new(Ok(
        AuthResult::new("AwesomeXboxer123".into(), "pic".into())
            .with_java_profile(Some("CoolBuilder42".into()), Some("java-uuid-1".into()))
    )));
    let client = support::build_test_client(db.clone(), fake).await;

    let resp = client.post("/api/auth/minecraft")
        .json(&serde_json::json!({ "code": "c", "redirect_uri": "http://app/cb" }))
        .dispatch().await;
    assert_eq!(resp.status(), rocket::http::Status::Ok);

    let uuid_alias: player_identity::Model = player_identity::Entity::find()
        .filter(player_identity::Column::Alias.eq("java-uuid-1"))
        .filter(player_identity::Column::AliasType.eq("platform_uuid"))
        .one(db.as_ref()).await.unwrap()
        .expect("uuid alias");
    assert_eq!(uuid_alias.player_id, xbl_id,
        "alias should re-point to operator-whitelisted XBL record");
}

#[tokio::test]
async fn b6_idempotent_relogin_does_not_duplicate_aliases() {
    use common::auth::AuthResult;
    use common::Game;
    use entity::player_identity;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

    let db = support::fresh_in_memory_db().await;
    let xbl_id = support::seed_player(&db.conn, "AwesomeXboxer123", &Game::Minecraft).await;
    support::seed_alias(&db.conn, xbl_id, "java-uuid-1", "platform_uuid", &Game::Minecraft).await;
    support::seed_alias(&db.conn, xbl_id, "CoolBuilder42", "minecraft_services", &Game::Minecraft).await;

    let make_fake = || std::sync::Arc::new(support::FakeMinecraftAuthenticator::new(Ok(
        AuthResult::new("AwesomeXboxer123".into(), "pic".into())
            .with_java_profile(Some("CoolBuilder42".into()), Some("java-uuid-1".into()))
    )));

    {
        let client = support::build_test_client(db.clone(), make_fake()).await;
        client.post("/api/auth/minecraft")
            .json(&serde_json::json!({ "code": "c", "redirect_uri": "http://app/cb" }))
            .dispatch().await;
    }
    {
        let client = support::build_test_client(db.clone(), make_fake()).await;
        client.post("/api/auth/minecraft")
            .json(&serde_json::json!({ "code": "c", "redirect_uri": "http://app/cb" }))
            .dispatch().await;
    }

    let aliases: Vec<player_identity::Model> = player_identity::Entity::find()
        .filter(player_identity::Column::PlayerId.eq(xbl_id))
        .all(db.as_ref()).await.unwrap();
    assert_eq!(aliases.len(), 2, "no duplicate aliases after relogin");
}

#[tokio::test]
async fn b7_alias_creation_failure_is_non_fatal() {
    use common::auth::AuthResult;
    use common::Game;
    use sea_orm::ConnectionTrait;

    let db = support::fresh_in_memory_db().await;
    let _xbl_id = support::seed_player(&db.conn, "AwesomeXboxer123", &Game::Minecraft).await;

    db.as_ref().execute_unprepared("DROP TABLE player_identity").await.unwrap();

    let fake = std::sync::Arc::new(support::FakeMinecraftAuthenticator::new(Ok(
        AuthResult::new("AwesomeXboxer123".into(), "pic".into())
            .with_java_profile(Some("CoolBuilder42".into()), Some("uuid".into()))
    )));
    let client = support::build_test_client(db.clone(), fake).await;

    let resp = client.post("/api/auth/minecraft")
        .json(&serde_json::json!({ "code": "c", "redirect_uri": "http://app/cb" }))
        .dispatch().await;
    assert_eq!(resp.status(), rocket::http::Status::Ok,
        "login still succeeds even if alias creation errors");
}

#[tokio::test]
async fn r1_full_round_trip_java_first_then_xbl_then_position() {
    use common::auth::AuthResult;
    use common::Game;
    use entity::player;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

    let db = support::fresh_in_memory_db().await;

    // Step 1: Java player joins via Fabric mod — auto-register creates the Java-first record.
    {
        let fake = std::sync::Arc::new(support::FakeMinecraftAuthenticator::new(Err(
            common::auth::AuthError::AuthenticationFailed("unused".into()),
        )));
        let client = support::build_test_client(db.clone(), fake).await;
        let body = minecraft_position_body("CoolBuilder42", Some("java-uuid-1"), None);
        let resp = client
            .post("/api/position")
            .header(rocket::http::Header::new("X-MC-Access-Token", "test-bypass-token"))
            .json(&body)
            .dispatch()
            .await;
        assert_eq!(resp.status(), rocket::http::Status::Ok);
    }

    // Verify Java-first state.
    {
        let players: Vec<player::Model> = player::Entity::find()
            .filter(player::Column::Game.eq(Game::Minecraft))
            .all(db.as_ref())
            .await
            .unwrap();
        assert_eq!(players.len(), 1);
        assert_eq!(players[0].gamertag.as_deref(), Some("CoolBuilder42"));
    }

    // Step 2: XBL login arrives — Phase B re-mapping creates new XBL record.
    {
        let fake = std::sync::Arc::new(support::FakeMinecraftAuthenticator::new(Ok(
            AuthResult::new("AwesomeXboxer123".into(), "pic".into())
                .with_java_profile(Some("CoolBuilder42".into()), Some("java-uuid-1".into())),
        )));
        let client = support::build_test_client(db.clone(), fake).await;
        let resp = client
            .post("/api/auth/minecraft")
            .json(&serde_json::json!({ "code": "c", "redirect_uri": "http://app/cb" }))
            .dispatch()
            .await;
        assert_eq!(resp.status(), rocket::http::Status::Ok);
    }

    // Verify two records (Java-first orphan + new XBL).
    {
        let players: Vec<player::Model> = player::Entity::find()
            .filter(player::Column::Game.eq(Game::Minecraft))
            .all(db.as_ref())
            .await
            .unwrap();
        assert_eq!(players.len(), 2);
        assert!(players.iter().any(|p| p.gamertag.as_deref() == Some("CoolBuilder42")));
        assert!(players.iter().any(|p| p.gamertag.as_deref() == Some("AwesomeXboxer123")));
    }

    // Step 3: Java player sends another position — UUID pre-resolution remaps to the XBL record.
    {
        let fake = std::sync::Arc::new(support::FakeMinecraftAuthenticator::new(Err(
            common::auth::AuthError::AuthenticationFailed("unused".into()),
        )));
        let client = support::build_test_client(db.clone(), fake).await;
        let body = minecraft_position_body("CoolBuilder42", Some("java-uuid-1"), None);
        let resp = client
            .post("/api/position")
            .header(rocket::http::Header::new("X-MC-Access-Token", "test-bypass-token"))
            .json(&body)
            .dispatch()
            .await;
        assert_eq!(resp.status(), rocket::http::Status::Ok);
    }

    // Final: still exactly two player records (no third created via duplicate auto-register).
    let players: Vec<player::Model> = player::Entity::find()
        .filter(player::Column::Game.eq(Game::Minecraft))
        .all(db.as_ref())
        .await
        .unwrap();
    assert_eq!(
        players.len(),
        2,
        "no third record should be created — UUID pre-resolution shadows the orphan"
    );
}

// link_java_identity requires mTLS (Certificate<'_> guard) which is not available
// in the test Rocket instance. The route itself has been refactored to use the
// MinecraftAuthenticator trait (Option A), but the mTLS guard prevents exercising
// it via the HTTP client in tests without a full TLS stack.
#[ignore]
#[tokio::test]
async fn link_java_endpoint_creates_minecraft_services_alias() {
    use common::auth::AuthResult;
    use common::Game;
    use entity::player_identity;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

    let db = support::fresh_in_memory_db().await;
    let xbl_id = support::seed_player(&db.conn, "AwesomeXboxer123", &Game::Minecraft).await;

    // authenticate_for_java_profile extracts minecraft_username from the canned AuthResult.
    let fake = std::sync::Arc::new(support::FakeMinecraftAuthenticator::new(Ok(
        AuthResult::new("ignored-gamertag".into(), "ignored-pic".into())
            .with_java_profile(Some("CoolBuilder42".into()), None),
    )));
    let client = support::build_test_client(db.clone(), fake).await;

    let body = serde_json::json!({
        "code": "auth-code",
        "redirect_uri": "http://app/cb",
        "client_id": "test-client-id",
        "gamertag": "AwesomeXboxer123"
    });
    let resp = client
        .post("/api/auth/link-java")
        .json(&body)
        .dispatch()
        .await;
    assert_eq!(resp.status(), rocket::http::Status::Ok);

    let aliases: Vec<player_identity::Model> = player_identity::Entity::find()
        .filter(player_identity::Column::PlayerId.eq(xbl_id))
        .filter(player_identity::Column::AliasType.eq("minecraft_services"))
        .all(db.as_ref())
        .await
        .unwrap();
    assert!(
        aliases.iter().any(|a| a.alias == "CoolBuilder42"),
        "/auth/link-java should create a minecraft_services alias for the Java username"
    );
}

#[tokio::test]
async fn b0_unknown_player_without_java_alias_is_rejected() {
    use common::auth::AuthResult;
    use sea_orm::EntityTrait;

    let db = support::fresh_in_memory_db().await;
    let fake = std::sync::Arc::new(support::FakeMinecraftAuthenticator::new(Ok(
        AuthResult::new("UnknownPlayer".into(), "pic".into())
            .with_java_profile(None, None)
    )));
    let client = support::build_test_client(db.clone(), fake).await;

    let resp = client.post("/api/auth/minecraft")
        .json(&serde_json::json!({ "code": "c", "redirect_uri": "http://app/cb" }))
        .dispatch().await;
    assert_eq!(resp.status(), rocket::http::Status::Forbidden,
        "non-whitelisted player without Java UUID match must be rejected");

    let players: Vec<entity::player::Model> = entity::player::Entity::find()
        .all(db.as_ref()).await.unwrap();
    assert_eq!(players.len(), 0, "no player should be created");
}
