use crate::{NetworkStreamManager, structs::app_state::AppState};
use common::response::LoginResponse;
use log::{error, info};
use std::net::{IpAddr, SocketAddr};
use tauri::State;
use tauri::async_runtime::Mutex;
use url::Url;

#[tauri::command]
pub(crate) async fn stop_network_stream(
    network_stream: State<'_, Mutex<NetworkStreamManager>>,
) -> Result<(), ()> {
    let mut network_stream = network_stream.lock().await;
    _ = network_stream.stop().await;
    Ok(())
}

#[tauri::command]
#[tracing::instrument(skip(state, network_stream, data), fields(server = %server))]
pub(crate) async fn change_network_stream(
    server: String,
    data: LoginResponse,
    state: State<'_, Mutex<AppState>>,
    network_stream: State<'_, Mutex<NetworkStreamManager>>,
) -> Result<(), String> {
    // Short state lock — release before network I/O
    {
        let mut state = state.lock().await;
        state.current_server = Some(server.clone());
    }

    // Parse URL to extract just the hostname (without port) for DNS lookup and SNI
    let server_fqdn = Url::parse(&server)
        .ok()
        .and_then(|u| u.host_str().map(|s| s.to_string()))
        .unwrap_or_else(|| {
            server
                .replace("https://", "")
                .replace("http://", "")
                .split(':')
                .next()
                .unwrap_or(&server)
                .to_string()
        });

    // Default to 443 if quic_connect_string is empty or invalid
    let port: u16 = data.quic_connect_string.parse().unwrap_or(443);

    let socket_addr = match tokio::net::lookup_host(format!("{}:{}", server_fqdn, port)).await {
        Ok(addrs) => {
            let resolved: Vec<SocketAddr> = addrs.collect();
            match resolved
                .iter()
                .find(|sa| matches!(sa.ip(), IpAddr::V4(_)))
                .or_else(|| resolved.first())
            {
                Some(addr) => *addr,
                None => {
                    error!("System DNS returned no IPs for {}", server_fqdn);
                    return Err("DNS_FAIL: System DNS returned no results".to_string());
                }
            }
        }
        Err(e) => {
            error!("System DNS resolution failed for {}: {}", server_fqdn, e);
            return Err(format!("DNS_FAIL: {}", e));
        }
    };

    let mut network_stream = network_stream.lock().await;
    _ = network_stream.stop().await;
    match network_stream
        .restart(
            server_fqdn.clone(),
            server.clone(),
            socket_addr,
            data.gamertag,
            data.certificate_ca,
            data.certificate,
            data.certificate_key,
        )
        .await
    {
        Ok(()) => {
            info!("Now streaming {}", server);
        }
        Err(e) => {
            error!("QUIC connection failed to {}: {:?}", server, e);
            return Err(format!("QUIC_FAIL: {}", e));
        }
    };

    Ok(())
}

#[tauri::command]
pub(crate) async fn reset_nsm(nsm: State<'_, Mutex<NetworkStreamManager>>) -> Result<(), ()> {
    let mut nsm = nsm.lock().await;
    _ = nsm.reset().await;
    Ok(())
}
