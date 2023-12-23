use keytar::*;

const BVC_SERVICE_NAME: &'static str = "BEDROCK_VOICE_CHAT";

#[tauri::command(async)]
pub(crate) async fn set_credential(key: String, value: String) -> bool {
    match set_password(BVC_SERVICE_NAME, &key, &value) {
        Ok(_) => true,
        Err(e) => {
            tracing::error!("{}", e.to_string());
            false
        }
    }
}

#[tauri::command(async)]
pub(crate) async fn get_credential(key: String) -> Result<String, bool> {
    match get_password(BVC_SERVICE_NAME, &key) {
        Ok(password) => match password.success {
            true => Ok(password.password),
            false => Err(false),
        },
        Err(e) => {
            tracing::error!("{}", e.to_string());
            Err(false)
        }
    }
}

#[tauri::command(async)]
pub(crate) async fn del_credential(key: String) -> bool {
    match delete_password(BVC_SERVICE_NAME, &key) {
        Ok(_) => true,
        Err(e) => {
            tracing::error!("{}", e.to_string());
            false
        }
    }
}
