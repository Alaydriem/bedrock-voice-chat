use common::structs::config::LoginResponse;
use crate::auth::login;

#[tauri::command(async)]
pub(crate) async fn server_login(
    server: String,
    code: String,
    redirect: String
) -> Result<LoginResponse, bool> {
    return login::server_login(server, code, redirect).await;
}