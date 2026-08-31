use bvc_server_lib::config::Audio;
use bvc_server_lib::services::{AudioFileError, AudioFileService};
use common::Game;
use entity::player;
use sea_orm::{ActiveModelTrait, ActiveValue};
use tempfile::TempDir;

use crate::harness::{DatabaseFixture, OggFixture};

struct Fixture {
    db: DatabaseFixture,
    config: Audio,
    player_id: i32,
    _storage: TempDir,
}

impl Fixture {
    async fn create() -> Self {
        let db = DatabaseFixture::create().await.expect("fixture");
        let storage = tempfile::tempdir().expect("audio storage dir");

        let now = common::ncryptflib::rocket::Utc::now().timestamp();
        let uploader = player::ActiveModel {
            id: ActiveValue::NotSet,
            gamertag: ActiveValue::Set(Some("Alaydriem".to_string())),
            gamerpic: ActiveValue::Set(None),
            certificate: ActiveValue::Set(String::new()),
            certificate_key: ActiveValue::Set(String::new()),
            banished: ActiveValue::Set(false),
            keypair: ActiveValue::Set(Vec::new()),
            signature: ActiveValue::Set(Vec::new()),
            created_at: ActiveValue::Set(now),
            updated_at: ActiveValue::Set(now),
            game: ActiveValue::Set(Game::Minecraft),
        }
        .insert(&db.connection)
        .await
        .expect("insert uploader");

        let config = Audio {
            file_path: storage.path().to_string_lossy().to_string(),
            ..Audio::default()
        };

        Self {
            db,
            config,
            player_id: uploader.id,
            _storage: storage,
        }
    }

    async fn upload(
        &self,
        bytes: Vec<u8>,
    ) -> Result<common::response::AudioFileResponse, AudioFileError> {
        AudioFileService::upload(
            &self.db.connection,
            self.player_id,
            "Alaydriem".to_string(),
            "minecraft".to_string(),
            bytes,
            Some("track.opus".to_string()),
            &self.config,
        )
        .await
    }
}

// The duration is what the jukebox schedules an eject against and what the client shows,
// and it comes from the final granule position less the pre-skip. Skipping the pre-skip
// subtraction overstates every track by the same few milliseconds, which no field
// comparison would catch.
#[tokio::test]
async fn the_recorded_duration_is_the_final_granule_less_the_pre_skip() {
    let f = Fixture::create().await;
    let frames = 5;
    let stream = OggFixture::opus_stream(frames, OggFixture::SAMPLES_PER_FRAME_20MS);

    let response = f.upload(stream).await.expect("upload");

    assert_eq!(
        response.duration_ms as u64,
        OggFixture::duration_ms(frames, OggFixture::SAMPLES_PER_FRAME_20MS),
    );
}

// The ten-minute ceiling is the only bound on how long a jukebox holds a playback slot, so
// a stream over it must be refused before anything is written to disk.
#[tokio::test]
async fn a_stream_over_ten_minutes_is_refused() {
    let f = Fixture::create().await;
    // 600.12 seconds: the shortest stream this fixture can build that clears the ceiling.
    let stream = OggFixture::opus_stream(5001, OggFixture::SAMPLES_PER_FRAME_120MS);

    let rejection = f.upload(stream).await.expect_err("refused");

    assert!(matches!(rejection, AudioFileError::AudioTooLong));
}

// A stream just inside the ceiling is accepted, or the bound is off by one page and every
// ten-minute track is rejected.
#[tokio::test]
async fn a_stream_just_inside_ten_minutes_is_accepted() {
    let f = Fixture::create().await;
    let stream = OggFixture::opus_stream(5000, OggFixture::SAMPLES_PER_FRAME_120MS);

    assert!(f.upload(stream).await.is_ok());
}

// Reached before the parser, because a payload that is not Ogg at all fails inside it with
// an error that reads like a corrupt upload rather than a wrong file type.
#[tokio::test]
async fn a_payload_that_is_not_ogg_is_refused_on_its_magic() {
    let f = Fixture::create().await;

    let rejection = f
        .upload(b"RIFF....WAVEfmt ".to_vec())
        .await
        .expect_err("refused");

    assert!(matches!(rejection, AudioFileError::InvalidFormat));
}
