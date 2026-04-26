//! Shared test fixtures for cross-cutting integration tests.

use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

use async_trait::async_trait;
use common::auth::{AuthError, AuthResult, MinecraftAuthenticator};
use migration::{Migrator, MigratorTrait};
use parking_lot::Mutex;
use reqwest::Url;
use sea_orm::{Database, DatabaseConnection};

static DB_COUNTER: AtomicU64 = AtomicU64::new(1);

/// A shared in-memory SQLite database for integration tests.
///
/// Both the Rocket AppDb pool and the PlayerIdentityService connect to the same
/// named SQLite shared-cache URL so writes from either path are visible to the
/// other and to test-side queries on `conn`.
#[derive(Clone)]
pub struct TestDb {
    pub conn: Arc<DatabaseConnection>,
    pub url: String,
}

impl AsRef<DatabaseConnection> for TestDb {
    fn as_ref(&self) -> &DatabaseConnection {
        self.conn.as_ref()
    }
}

pub async fn fresh_in_memory_db() -> TestDb {
    let id = DB_COUNTER.fetch_add(1, Ordering::Relaxed);
    let url = format!("sqlite:file:bvc_test_{}?mode=memory&cache=shared", id);

    let conn = Database::connect(url.as_str())
        .await
        .expect("connect to named sqlite memory");
    Migrator::up(&conn, None)
        .await
        .expect("run migrations against in-memory sqlite");

    TestDb {
        conn: Arc::new(conn),
        url,
    }
}

pub struct FakeMinecraftAuthenticator {
    canned: Mutex<Option<Result<AuthResult, AuthError>>>,
}

impl FakeMinecraftAuthenticator {
    pub fn new(result: Result<AuthResult, AuthError>) -> Self {
        Self { canned: Mutex::new(Some(result)) }
    }
}

#[async_trait]
impl MinecraftAuthenticator for FakeMinecraftAuthenticator {
    async fn authenticate(
        &self,
        _code: String,
        _redirect_uri: Url,
    ) -> Result<AuthResult, AuthError> {
        self.canned
            .lock()
            .take()
            .expect("FakeMinecraftAuthenticator: authenticate called more than once or unset")
    }

    async fn authenticate_for_java_profile(
        &self,
        _code: String,
        _redirect_uri: Url,
    ) -> Result<String, AuthError> {
        let result = self
            .canned
            .lock()
            .take()
            .expect("FakeMinecraftAuthenticator: authenticate_for_java_profile called more than once or unset");
        result.and_then(|auth| {
            auth.minecraft_username.ok_or(AuthError::AuthenticationFailed(
                "no java username in canned AuthResult".into(),
            ))
        })
    }
}

pub async fn seed_player(
    conn: &Arc<DatabaseConnection>,
    gamertag: &str,
    game: &common::Game,
) -> i32 {
    use sea_orm::{ActiveModelTrait, ActiveValue};
    let m = entity::player::ActiveModel {
        gamertag: ActiveValue::Set(Some(gamertag.into())),
        gamerpic: ActiveValue::Set(None),
        certificate: ActiveValue::Set(String::new()),
        certificate_key: ActiveValue::Set(String::new()),
        banished: ActiveValue::Set(false),
        keypair: ActiveValue::Set(vec![0u8; 64]),
        signature: ActiveValue::Set(vec![0u8; 96]),
        created_at: ActiveValue::Set(0),
        updated_at: ActiveValue::Set(0),
        game: ActiveValue::Set(game.clone()),
        ..Default::default()
    };
    let inserted = m.insert(conn.as_ref()).await.expect("seed player insert");
    inserted.id
}

pub async fn seed_alias(
    conn: &Arc<DatabaseConnection>,
    player_id: i32,
    alias: &str,
    alias_type: &str,
    game: &common::Game,
) {
    use sea_orm::{ActiveModelTrait, ActiveValue};
    let m = entity::player_identity::ActiveModel {
        player_id: ActiveValue::Set(player_id),
        alias: ActiveValue::Set(alias.into()),
        alias_type: ActiveValue::Set(alias_type.into()),
        game: ActiveValue::Set(game.clone()),
        created_at: ActiveValue::Set(0),
        updated_at: ActiveValue::Set(0),
        ..Default::default()
    };
    m.insert(conn.as_ref()).await.expect("seed alias insert");
}

pub async fn build_test_client(
    db: TestDb,
    authenticator: Arc<dyn MinecraftAuthenticator>,
) -> rocket::local::asynchronous::Client {
    let rocket = bvc_server_lib::build_test_rocket(
        authenticator,
        db.url.clone(),
    )
    .await;

    rocket::local::asynchronous::Client::tracked(rocket)
        .await
        .expect("test rocket client")
}
