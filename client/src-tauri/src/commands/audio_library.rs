use common::response::AudioFileResponse;
use common::response::auth::AuthStateResponse;

use crate::audio::encode::AudioFileEncoder;
use crate::structs::app_state::AppState;
use tauri::{async_runtime::Mutex, State};

/// Upload an audio file: encode to Ogg/Opus then upload to server.
#[tauri::command(async)]
pub(crate) async fn upload_audio_file(
    app_state: State<'_, Mutex<AppState>>,
    #[allow(non_snake_case)]
    filePath: String,
    server: Option<String>,
    game: Option<String>,
) -> Result<AudioFileResponse, String> {
    // Encode file to Ogg/Opus (blocking operation)
    let path = filePath.clone();
    let encode_result = tokio::task::spawn_blocking(move || AudioFileEncoder::encode(&path))
        .await
        .map_err(|e| format!("Task join error: {}", e))?
        .map_err(|e| format!("Encoding failed: {}", e))?;

    let state = app_state.lock().await;
    let api = match server {
        Some(endpoint) => {
            drop(state);
            app_state
                .lock()
                .await
                .get_api_client_for_server(&endpoint)
                .await?
        }
        None => state.get_api_client()?.clone(),
    };

    api.upload_audio_file(encode_result.opus_bytes, &encode_result.original_filename, game.as_deref())
        .await
}

/// List all audio files on the server.
#[tauri::command(async)]
pub(crate) async fn list_audio_files(
    app_state: State<'_, Mutex<AppState>>,
    server: Option<String>,
    game: Option<String>,
) -> Result<Vec<AudioFileResponse>, String> {
    let state = app_state.lock().await;
    let api = match server {
        Some(endpoint) => {
            drop(state);
            app_state
                .lock()
                .await
                .get_api_client_for_server(&endpoint)
                .await?
        }
        None => state.get_api_client()?.clone(),
    };

    api.list_audio_files(game.as_deref()).await
}

/// Delete an audio file from the server.
#[tauri::command(async)]
pub(crate) async fn delete_audio_file(
    app_state: State<'_, Mutex<AppState>>,
    #[allow(non_snake_case)]
    fileId: String,
    server: Option<String>,
    game: Option<String>,
) -> Result<bool, String> {
    let state = app_state.lock().await;
    let api = match server {
        Some(endpoint) => {
            drop(state);
            app_state
                .lock()
                .await
                .get_api_client_for_server(&endpoint)
                .await?
        }
        None => state.get_api_client()?.clone(),
    };

    api.delete_audio_file(&fileId, game.as_deref()).await
}

/// Refresh server permissions for the authenticated player.
/// If the server returns re-issued certificates (due to old CN format or expiry),
/// the API client is updated in-memory with the new identity.
#[tauri::command(async)]
pub(crate) async fn refresh_server_state(
    app_state: State<'_, Mutex<AppState>>,
    server: Option<String>,
    game: Option<String>,
) -> Result<AuthStateResponse, String> {
    // Get the API client and drop the lock before the network call
    let api = {
        let state = app_state.lock().await;
        match server {
            Some(ref endpoint) => state.get_api_client_for_server(endpoint).await?,
            None => state.get_api_client()?.clone(),
        }
    };

    let response = api.get_server_state(game.as_deref()).await?;

    // If the server sent re-issued certs, update the API client in-memory
    if let (Some(cert), Some(key)) = (&response.certificate, &response.certificate_key) {
        let new_api = api.with_new_identity(cert, key);
        let mut state = app_state.lock().await;
        if let Some(endpoint) = state.current_server.clone() {
            state.api_client = Some(new_api.clone());
            let mut pool = state.server_pool.write().await;
            pool.insert(endpoint, new_api);
        }
    }

    Ok(response)
}
