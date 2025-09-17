use crate::auth::login;
use crate::structs::app_state::AppState;
use common::structs::config::LoginResponse;
use tauri::{async_runtime::Mutex, State};

#[tauri::command(async)]
pub(crate) async fn server_login(
    app_state: State<'_, Mutex<AppState>>,
    server: String,
    code: String,
    redirect: String,
) -> Result<LoginResponse, bool> {
    let login_result = login::server_login(server.clone(), code, redirect).await;
    
    // If login is successful, initialize the API client
    if let Ok(ref response) = login_result {
        let mut state = app_state.lock().await;
        state.initialize_api_client(
            server,
            response.certificate_ca.clone(),
            response.certificate.clone() + &response.certificate_key.clone(),
        );
    }
    
    login_result
}
