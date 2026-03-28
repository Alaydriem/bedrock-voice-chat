use crate::{NetworkStreamManager, structs::app_state::AppState};
use common::response::LoginResponse;
use log::{error, info};
use std::net::SocketAddr;
use tauri::State;
use tauri::async_runtime::Mutex;
use trust_dns_resolver::{
    Resolver, TokioAsyncResolver,
    config::{ResolverConfig, ResolverOpts},
};
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

    // Cloudflare DNS (async), then system resolver fallback (spawn_blocking)
    let resolver = TokioAsyncResolver::tokio(ResolverConfig::cloudflare(), ResolverOpts::default());
    let socket_addr = match resolver.lookup_ip(server_fqdn.clone()).await {
        Ok(response) => match response.iter().next() {
            Some(ip) => SocketAddr::new(ip, port),
            None => {
                error!("Cloudflare DNS lookup returned no IPs for {}", server_fqdn);
                return Err("DNS_FAIL: DNS lookup returned no results".to_string());
            }
        },
        Err(cf_err) => {
            info!("Cloudflare DNS failed for {}: {}. Trying system resolver.", server_fqdn, cf_err);
            let fqdn = server_fqdn.clone();
            match tokio::task::spawn_blocking(move || {
                let resolver = Resolver::from_system_conf().map_err(|e| e.to_string())?;
                let response = resolver.lookup_ip(&fqdn).map_err(|e| e.to_string())?;
                response
                    .iter()
                    .next()
                    .map(|ip| SocketAddr::new(ip, port))
                    .ok_or_else(|| "System DNS returned no IPs".to_string())
            })
            .await
            {
                Ok(Ok(addr)) => addr,
                Ok(Err(e)) => {
                    error!("System DNS resolution failed for {}: {}", server_fqdn, e);
                    return Err(format!("DNS_FAIL: {}", e));
                }
                Err(e) => {
                    error!("System DNS task failed: {:?}", e);
                    return Err("DNS_FAIL: Could not resolve server address".to_string());
                }
            }
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
