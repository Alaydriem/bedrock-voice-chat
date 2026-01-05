use crate::audio::AudioPacket;
use crate::network::NetworkPacket;
use crate::{structs::app_state::AppState, NetworkStreamManager};
use common::structs::config::LoginResponse;
use flume::{Receiver, Sender};
use log::{error, info};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tauri::async_runtime::Mutex;
use tauri::{AppHandle, Manager, State};
use trust_dns_resolver::{
    config::{ResolverConfig, ResolverOpts},
    Resolver, TokioAsyncResolver,
};

#[tauri::command]
pub(crate) async fn stop_network_stream(
    network_stream: State<'_, Mutex<NetworkStreamManager>>,
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
    network_stream: State<'_, Mutex<NetworkStreamManager>>,
) -> Result<(), ()> {
    let mut state = state.lock().await;
    state.current_server = Some(server.clone());

    let server_fqdn = server.clone().replace("https://", "");

    let resolver = TokioAsyncResolver::tokio(ResolverConfig::cloudflare(), ResolverOpts::default());

    // Try a DNS lookup for the network, then fallback to the /etc/hosts on the machine
    let socket_addr = match resolver.lookup_ip(server_fqdn.clone()).await {
        Ok(response) => match response.iter().next() {
            Some(ip) => SocketAddr::new(ip, data.quic_connect_string.parse().unwrap()),
            None => {
                error!("TrustDNS Lookup was successful, but no IP's were returned. Networking issue, restart BVC.");
                return Err(());
            }
        },
        Err(_) => match Resolver::from_system_conf() {
            Ok(resolver) => match resolver.lookup_ip(server_fqdn.clone()) {
                Ok(response) => match response.iter().next() {
                    Some(ip) => SocketAddr::new(ip, data.quic_connect_string.parse().unwrap()),
                    None => {
                        error!("TrustDNS Lookup was successful, but no IP's were returned. Networking issue, restart BVC.");
                        return Err(());
                    }
                },
                Err(e) => {
                    error!("{:?}", e);
                    return Err(());
                }
            },
            Err(e) => {
                error!("{:?}", e);
                return Err(());
            }
        },
    };

    let mut network_stream = network_stream.lock().await;
    match network_stream
        .restart(
            server_fqdn.clone(),
            socket_addr,
            data.gamertag,
            data.certificate_ca,
            data.certificate,
            data.certificate_key,
        )
        .await
    {
        Ok(()) => {
            info!("Now streaming {}", server.clone());
        }
        Err(e) => {
            error!(
                "Failed to re-initialize network stream: {:?} {}",
                e,
                e.to_string()
            );
            return Err(());
        }
    };

    Ok(())
}

#[tauri::command]
pub(crate) async fn reset_nsm(
    nsm: State<'_, Mutex<NetworkStreamManager>>,
) -> Result<(), ()> {
    let mut nsm = nsm.lock().await;
    _ = nsm.reset().await;
    Ok(())
}
