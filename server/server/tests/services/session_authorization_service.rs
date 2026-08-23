use bvc_server_lib::services::{
    CertificateRevocationService, SessionAuthorizationService, SessionRejection,
};
use common::Game;
use entity::player;
use sea_orm::{ActiveModelTrait, ActiveValue, ColumnTrait, EntityTrait, QueryFilter};
use crate::harness::{CertificateFixture, DatabaseFixture};

struct Fixture {
    db: DatabaseFixture,
    certificates: CertificateFixture,
    service: SessionAuthorizationService,
}

impl Fixture {
    async fn create() -> Self {
        let certificates = CertificateFixture::create().expect("certificate fixture");
        let db = DatabaseFixture::create().await.expect("fixture");
        let service = SessionAuthorizationService::new(CertificateRevocationService::new_shared());
        Self {
            db,
            certificates,
            service,
        }
    }

    // The leaf DER a handshake would present for this player.
    fn leaf_der(&self, gamertag: &str, game: &Game) -> Vec<u8> {
        let (cert, _key) = self
            .certificates
            .service
            .sign_player_cert(gamertag, game)
            .expect("sign");
        cert.der().to_vec()
    }

    async fn insert_player(&self, gamertag: &str, game: &Game) {
        let now = common::ncryptflib::rocket::Utc::now().timestamp();
        let active = player::ActiveModel {
            id: ActiveValue::NotSet,
            gamertag: ActiveValue::Set(Some(gamertag.to_string())),
            gamerpic: ActiveValue::Set(None),
            certificate: ActiveValue::Set(String::new()),
            certificate_key: ActiveValue::Set(String::new()),
            banished: ActiveValue::Set(false),
            keypair: ActiveValue::Set(Vec::new()),
            signature: ActiveValue::Set(Vec::new()),
            created_at: ActiveValue::Set(now),
            updated_at: ActiveValue::Set(now),
            game: ActiveValue::Set(game.clone()),
        };
        active.insert(&self.db.connection).await.expect("insert");
    }

    async fn banish(&self, gamertag: &str) {
        let found = player::Entity::find()
            .filter(player::Column::Gamertag.eq(gamertag))
            .one(&self.db.connection)
            .await
            .expect("find")
            .expect("player");
        let mut active: player::ActiveModel = found.into();
        active.banished = ActiveValue::Set(true);
        active.update(&self.db.connection).await.expect("update");
    }
}

#[tokio::test]
async fn a_registered_player_is_authorized() {
    let f = Fixture::create().await;
    f.insert_player("Steve", &Game::Minecraft).await;
    let der = f.leaf_der("Steve", &Game::Minecraft);

    let player = f
        .service
        .authorize(&f.db.connection, &der)
        .await
        .expect("authorized");

    assert_eq!(player.gamertag.as_deref(), Some("Steve"));
}

// The QUIC accept loop previously parsed the CN and trusted it, so any certificate this CA
// had ever signed opened a voice session regardless of whether the player still existed.
#[tokio::test]
async fn a_certificate_with_no_player_row_is_refused() {
    let f = Fixture::create().await;
    let der = f.leaf_der("NeverRegistered", &Game::Minecraft);

    let rejection = f
        .service
        .authorize(&f.db.connection, &der)
        .await
        .expect_err("refused");

    assert_eq!(rejection, SessionRejection::UnknownPlayer);
}

#[tokio::test]
async fn a_banished_player_is_refused() {
    let f = Fixture::create().await;
    f.insert_player("Griefer", &Game::Minecraft).await;
    f.banish("Griefer").await;
    let der = f.leaf_der("Griefer", &Game::Minecraft);

    let rejection = f
        .service
        .authorize(&f.db.connection, &der)
        .await
        .expect_err("refused");

    assert_eq!(rejection, SessionRejection::Banished);
}

#[tokio::test]
async fn a_revoked_certificate_is_refused() {
    let f = Fixture::create().await;
    f.insert_player("Steve", &Game::Minecraft).await;
    let der = f.leaf_der("Steve", &Game::Minecraft);

    let fingerprint = common::structs::certificate::CertificateFingerprint::from_der(&der);
    f.service
        .revocations()
        .revoke(
            &f.db.connection,
            &fingerprint,
            None,
            "test",
            4_102_444_800,
        )
        .await
        .expect("revoke");

    let rejection = f
        .service
        .authorize(&f.db.connection, &der)
        .await
        .expect_err("refused");

    assert_eq!(rejection, SessionRejection::Revoked);
}

// A CN that is not a known-game player must never reach the player path, which is the
// property `ConnectionClassifier` exists to hold.
#[tokio::test]
async fn a_non_player_common_name_is_refused() {
    let f = Fixture::create().await;

    let mut dn = rcgen::DistinguishedName::new();
    dn.push(
        rcgen::DnType::CommonName,
        "server::relay.example.com:5000".to_string(),
    );
    let mut params = rcgen::CertificateParams::default();
    params.distinguished_name = dn;
    let key_pair = rcgen::KeyPair::generate().expect("keypair");
    let cert = params.self_signed(&key_pair).expect("self-signed");

    let rejection = f
        .service
        .authorize(&f.db.connection, cert.der())
        .await
        .expect_err("refused");

    assert_eq!(rejection, SessionRejection::NotAPlayer);
}
