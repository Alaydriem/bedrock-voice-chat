use crate::analytics::AnalyticsService;
use crate::{NetworkStreamManager, structs::app_state::AppState};
use common::response::LoginResponse;
use common::structs::network::QuicPortSelection;
use log::{error, info, warn};
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use tauri::State;
use tauri::async_runtime::Mutex;
use url::Url;

#[tauri::command]
pub(crate) async fn stop_network_stream(
    network_stream: State<'_, Mutex<NetworkStreamManager>>,
    analytics: State<'_, Arc<AnalyticsService>>,
) -> Result<(), ()> {
    let mut network_stream = network_stream.lock().await;
    _ = network_stream.stop().await;
    analytics.clear_connected_server();
    analytics.clear_player();
    Ok(())
}

#[tauri::command]
#[tracing::instrument(skip(state, network_stream, data, analytics), fields(server = %server))]
pub(crate) async fn change_network_stream(
    server: String,
    data: LoginResponse,
    state: State<'_, Mutex<AppState>>,
    network_stream: State<'_, Mutex<NetworkStreamManager>>,
    analytics: State<'_, Arc<AnalyticsService>>,
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

    // The keyring copy of the port is frozen at login. Ask the server what it
    // currently advertises so a server-side port change does not strand
    // already-authenticated clients; the stored value is the fallback for when the
    // config fetch itself fails.
    let api = {
        let state = state.lock().await;
        state.get_api_client_for_server(&server).await
    };

    let advertised = match api {
        Ok(api) => match api.get_config().await {
            Ok(config) => Some((config.quic_ports, config.quic_port)),
            Err(e) => {
                warn!("Config fetch failed for {}; using stored port: {}", server, e);
                None
            }
        },
        Err(e) => {
            warn!("No API client for {}; using stored port: {}", server, e);
            None
        }
    };

    let (advertised_ports, advertised_scalar) = advertised.unwrap_or((Vec::new(), 0));
    let ports = QuicPortSelection::resolve(
        &advertised_ports,
        advertised_scalar,
        Some(&data.quic_connect_string),
    );

    info!("QUIC candidate ports for {}: {:?}", server_fqdn, ports);

    // One DNS lookup serves every candidate: the host is identical and only the
    // port varies, so resolving per port would repeat the same query.
    let resolved_ip = match tokio::net::lookup_host(format!("{}:{}", server_fqdn, ports[0])).await {
        Ok(addrs) => {
            let resolved: Vec<SocketAddr> = addrs.collect();
            match resolved
                .iter()
                .find(|sa| matches!(sa.ip(), IpAddr::V4(_)))
                .or_else(|| resolved.first())
            {
                Some(addr) => addr.ip(),
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

    let socket_addrs: Vec<SocketAddr> = ports
        .iter()
        .map(|port| SocketAddr::new(resolved_ip, *port))
        .collect();

    let mut network_stream = network_stream.lock().await;
    _ = network_stream.stop().await;
    analytics.clear_connected_server();
    analytics.clear_player();
    let gamertag = data.gamertag.clone();
    match network_stream
        .restart(
            server_fqdn.clone(),
            server.clone(),
            socket_addrs,
            data.gamertag,
            data.certificate_ca,
            data.certificate,
            data.certificate_key,
        )
        .await
    {
        Ok(()) => {
            info!("Now streaming {}", server);
            analytics.set_connected_server(Some(server.clone()));
            analytics.set_player(&gamertag);
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
