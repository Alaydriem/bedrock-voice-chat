use rocket::fs::NamedFile;
use rocket::State;
use rocket_okapi::openapi;

use crate::config::Audio;
use crate::services::AudioStreamTokenCache;

#[openapi(skip)]
#[get("/stream?<token>")]
pub async fn audio_file_stream(
    token: &str,
    token_cache: &State<AudioStreamTokenCache>,
    config: &State<Audio>,
) -> Option<NamedFile> {
    let file_id = token_cache.validate_token(token).await?;
    let path = format!("{}/{}.opus", config.file_path, file_id);
    NamedFile::open(path).await.ok()
}
