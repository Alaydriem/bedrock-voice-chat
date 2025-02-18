use std::{net::SocketAddr, sync::Mutex};
use common::structs::config::LoginResponse;
use tauri::{AppHandle, Emitter, State};
use async_mutex::Mutex as AsyncMutex;

use crate::{
    network::events::{ChangeNetworkStreamEvent, StopNetworkStreamEvent},
    structs::app_state::AppState, NetworkStreamManager
};
use log::{info, error};

#[tauri::command]
pub(crate) async fn stop_network_stream(
    app: AppHandle
) -> Result<(), ()> {
    _ = app.emit("stop-network-stream", StopNetworkStreamEvent {});
    Ok(())
}

#[tauri::command]
pub(crate) async fn change_network_stream(
    server: String,
    data: LoginResponse,
    app: AppHandle,
    state: State<'_, Mutex<AppState>>,
    network_stream: State<'_, AsyncMutex<NetworkStreamManager>>
) -> Result<(), ()> {
    // Stop the network stream
    _ = app.emit("stop-network-stream", StopNetworkStreamEvent {});

    let event = match state.lock() {
        Ok(mut state) => {
            // Update the current server
            state.current_server = Some(server.clone());
            // Change and restart the network stream
            ChangeNetworkStreamEvent {
                server: server.clone(),
                socket: data.quic_connect_string,
                name: data.gamertag,
                ca_cert: data.certificate_ca,
                cert: data.certificate,
                key: data.certificate_key
            }
        },
        Err(e) => {
            error!("Failed to access AppState in `change-network-stream` {:?}", e);
            return Err(());
        }
    };

    _ = app.emit("change-network-stream", event.clone());

    let mut network_stream = network_stream.lock().await;
    match network_stream.restart(
        event.server,
        event.socket.parse::<SocketAddr>().unwrap(),
        event.name,
        event.ca_cert,
        event.cert,
        event.key
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
