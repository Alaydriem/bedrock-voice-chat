use crate::api::Api;

#[tauri::command(async)]
pub(crate) async fn api_ping(endpoint: String, cert: String, pem: String) -> Result<(), bool> {
    let api = Api::new(endpoint, cert.clone(), pem.clone());
    return api.ping().await;
}
