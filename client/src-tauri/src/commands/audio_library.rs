use common::request::AudioFileListQuery;
use common::response::{AudioFileResponse, PaginatedResponse};

use crate::audio::encode::AudioFileEncoder;
use crate::keyring::KeyringService;
use crate::structs::app_state::AppState;
use tauri::{State, async_runtime::Mutex};

#[tauri::command(async)]
pub(crate) async fn upload_audio_file(
    app_state: State<'_, Mutex<AppState>>,
    #[allow(non_snake_case)] filePath: String,
    server: Option<String>,
    game: Option<String>,
) -> Result<AudioFileResponse, String> {
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

    api.upload_audio_file(
        encode_result.opus_bytes,
        &encode_result.original_filename,
        game.as_deref(),
    )
    .await
}

#[tauri::command(async)]
pub(crate) async fn list_audio_files(
    app_state: State<'_, Mutex<AppState>>,
    server: Option<String>,
    game: Option<String>,
    query: Option<AudioFileListQuery>,
) -> Result<PaginatedResponse<AudioFileResponse>, String> {
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

    let query = query.unwrap_or_default();
    api.list_audio_files(game.as_deref(), &query).await
}

#[tauri::command(async)]
pub(crate) async fn delete_audio_file(
    app_state: State<'_, Mutex<AppState>>,
    #[allow(non_snake_case)] fileId: String,
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

#[tauri::command(async)]
pub(crate) async fn refresh_server_state(
    app_state: State<'_, Mutex<AppState>>,
    keyring: State<'_, Mutex<KeyringService>>,
    server: Option<String>,
    game: Option<String>,
) -> Result<common::response::auth::AuthStateResponse, String> {
    let state = app_state.lock().await;
    let current_server = state.current_server.clone();
    let api = match server {
        Some(ref endpoint) => {
            drop(state);
            app_state
                .lock()
                .await
                .get_api_client_for_server(endpoint)
                .await?
        }
        None => state.get_api_client()?.clone(),
    };

    let response = api.get_server_state(game.as_deref()).await?;

    // Persist refreshed data to keyring
    let target_server = server.or(current_server);
    if let Some(ref server_url) = target_server {
        let mut kr = keyring.lock().await;
        if let Ok(perms_json) = serde_json::to_string(&response.server_permissions) {
            let _ = kr.set_credential(server_url, "server_permissions", &perms_json);
        }
        if let Some(ref cert) = response.certificate {
            let _ = kr.set_credential(server_url, "certificate", cert);
        }
        if let Some(ref cert_key) = response.certificate_key {
            let _ = kr.set_credential(server_url, "certificate_key", cert_key);
        }
    }

    Ok(response)
}

#[tauri::command(async)]
pub(crate) async fn get_audio_stream_url(
    app_state: State<'_, Mutex<AppState>>,
    #[allow(non_snake_case)] fileId: String,
    server: Option<String>,
    game: Option<String>,
) -> Result<String, String> {
    let state = app_state.lock().await;
    let endpoint = state.current_server.clone().unwrap_or_default();
    let api = match server {
        Some(ref ep) => {
            drop(state);
            app_state
                .lock()
                .await
                .get_api_client_for_server(ep)
                .await?
        }
        None => state.get_api_client()?.clone(),
    };

    let token_response = api
        .get_audio_stream_token(&fileId, game.as_deref())
        .await?;

    let base = server.unwrap_or(endpoint);
    Ok(format!(
        "{}/api/audio/stream?token={}",
        base, token_response.token
    ))
}
