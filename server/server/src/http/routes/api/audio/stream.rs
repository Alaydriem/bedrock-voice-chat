use rocket::State;
use rocket::fs::NamedFile;
use rocket::http::Status;
use rocket_okapi::openapi;

use crate::config::Audio;
use crate::services::AudioStreamTokenCache;

// Skipped in the OpenAPI spec: returns a `NamedFile` byte stream, which okapi
// cannot derive a response schema for. Documented in the wiki instead.
#[openapi(skip)]
#[get("/stream?<token>")]
pub async fn audio_file_stream(
    token: &str,
    token_cache: &State<AudioStreamTokenCache>,
    config: &State<Audio>,
) -> Result<NamedFile, Status> {
    let file_id = token_cache
        .validate_token(token)
        .await
        .ok_or(Status::NotFound)?;
    let path = format!("{}/{}.opus", config.file_path, file_id);
    NamedFile::open(path).await.map_err(|_| Status::NotFound)
}
