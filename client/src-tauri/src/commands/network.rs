use std::net::SocketAddr;
use common::structs::config::LoginResponse;
use tauri::State;
use tauri::async_runtime::Mutex;

use crate::{
    structs::app_state::AppState, NetworkStreamManager
};
use log::{info, error};

#[tauri::command]
pub(crate) async fn stop_network_stream(
    network_stream: State<'_, Mutex<NetworkStreamManager>>
) -> Result<(), ()> {
    let mut network_stream = network_stream.lock().await;
    _ = network_stream.stop();
    Ok(())
}

#[tauri::command]
pub(crate) async fn change_network_stream(
    server: String,
    data: LoginResponse,
    state: State<'_, Mutex<AppState>>,
    network_stream: State<'_, Mutex<NetworkStreamManager>>
) -> Result<(), ()> {
    let mut state = state.lock().await;
    state.current_server = Some(server.clone());

    let mut network_stream = network_stream.lock().await;
    match network_stream.restart(
        server.clone(),
        data.quic_connect_string.parse::<SocketAddr>().unwrap(),
        data.gamertag,
        data.certificate_ca,
        data.certificate,
        data.certificate_key
    ).await {
        Ok(()) => {
            info!("Now streaming {}", server.clone());
        },
        Err(e) => {
            error!("Failed to re-initialize network stream: {:?} {}", e, e.to_string());
            return Err(());
        }
    };
    
    Ok(())
}
