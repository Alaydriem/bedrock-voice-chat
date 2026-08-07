use crate::analytics::AnalyticsService;
use crate::{NetworkStreamManager, structs::app_state::AppState};
use common::consts::version::PROTOCOL_VERSION;
use common::net::{CandidatePlan, ReachabilityPlanner};
use common::response::api::config::ProtocolCompatibility;
use common::response::LoginResponse;
use common::structs::reachability::ServerReachability;
use log::{error, info, warn};
use std::sync::Arc;
use tauri::State;
use tauri::async_runtime::Mutex;

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
    app: tauri::AppHandle,
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

    // The keyring copy of the port is frozen at login. Ask the server what it
    // currently advertises so a server-side port change does not strand
    // already-authenticated clients; the stored value is the fallback for when the
    // config fetch itself fails.
    let api = {
        let state = state.lock().await;
        state.get_api_client_for_server(&server).await
    };

    let api_for_invalidation = api.as_ref().ok().cloned();

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

    let request = match ReachabilityPlanner::plan(
        &server,
        &advertised_ports,
        advertised_scalar,
        Some(&data.quic_connect_string),
    )
    .await
    {
        Ok(request) => request,
        Err(e) => {
            error!("Reachability planning failed for {}: {}", server, e);
            return Err(format!("{}", e));
        }
    };

    let server_fqdn = request.host.clone();
    let resolved = request.addrs.clone();
    let ports = request.quic_ports.clone();

    info!("QUIC candidate ports for {}: {:?}", server_fqdn, ports);

    let (reachability, family_preference) = {
        let state = state.lock().await;
        (state.reachability(), state.family_preference())
    };

    let report = reachability.evaluate(&request).await;
    family_preference.set(report.preference());

    let plan = CandidatePlan::build(&resolved, &ports, &report);

    info!(
        "QUIC candidates for {} ({:?}): {:?}",
        server_fqdn,
        report.preference(),
        plan.candidates()
            .iter()
            .map(|c| c.dial())
            .collect::<Vec<_>>()
    );

    let mut network_stream = network_stream.lock().await;
    _ = network_stream.stop().await;
    analytics.clear_connected_server();
    analytics.clear_player();
    let gamertag = data.gamertag.clone();
    // The canonical identity, composed from the same two fields the server put in the
    // certificate CN. Every control report the client sends names itself with this, and the
    // server compares it against the CN — a bare gamertag is dropped silently.
    let identity = data
        .game
        .clone()
        .unwrap_or(common::Game::Minecraft)
        .membership_key(&gamertag);
    // The error is rendered to a String before any further await: it is a
    // `Box<dyn Error>`, which is not Send, and holding one across an await would
    // make this command's future non-Send.
    let outcome = network_stream
        .restart(
            server_fqdn.clone(),
            server.clone(),
            plan,
            identity,
            data.certificate_ca,
            data.certificate,
            data.certificate_key,
        )
        .await
        .map_err(|e| format!("{:?}", e));

    match outcome {
        Ok(()) => {
            info!("Now streaming {}", server);
            analytics.set_connected_server(Some(server.clone()));
            analytics.set_player(&gamertag);
        }
        Err(detail) => {
            error!("QUIC connection failed to {}: {}", server, detail);
            // A verdict that led nowhere must not persist to its TTL: the next
            // attempt re-probes rather than repeating the same ordering.
            reachability.invalidate(&server_fqdn).await;
            // The advertised port is a verdict too. A connect that failed because the
            // server moved it would otherwise be handed the same stale answer for the
            // rest of the window.
            if let Some(api) = api_for_invalidation {
                api.invalidate_config().await;
            }
            return Err(format!("QUIC_FAIL: {}", detail));
        }
    };

    // Per-player volumes are keyed `(server, cn)`, so the projection the mixer holds belongs
    // to whichever server was current before this. Re-seeded here, at the one place that
    // authoritatively changes `AppState.current_server`, rather than from a particular
    // navigation in the webview — that way the cold start, a reconnect and any future switch
    // path all get it without each having to remember.
    drop(network_stream);
    crate::players::PlayerSettingsCoordinator::reseed(&app).await;

    Ok(())
}

// Runs before login, which is what the probe is built for: a Version Negotiation
// reply or a rejected handshake both prove a live listener without a client
// certificate, and neither exists yet at this point in the flow.
//
// The probe instance is the one change_network_stream reads, so a measurement made
// while the address is being typed warms the connect path rather than duplicating
// it.
#[tauri::command]
pub(crate) async fn probe_server(
    server: String,
    quic_ports: Vec<u32>,
    quic_port: u32,
    state: State<'_, Mutex<AppState>>,
) -> Result<ServerReachability, String> {
    let request = ReachabilityPlanner::plan(&server, &quic_ports, quic_port, None)
        .await
        .map_err(|e| e.to_string())?;

    let reachability = {
        let state = state.lock().await;
        state.reachability()
    };

    Ok(reachability.evaluate(&request).await)
}

// Compatibility is a different axis from reachability, and both are needed before a
// player commits to a server: one answers "can voice get there", the other "will this
// build understand it". Credential-free on purpose — the address field asks this of a
// server nobody has signed into yet, so it takes the version the caller already read
// from the unauthenticated /api/config rather than fetching it again.
#[tauri::command]
pub(crate) fn check_protocol_compatibility(server_version: String) -> ProtocolCompatibility {
    ProtocolCompatibility::between(&server_version, PROTOCOL_VERSION)
}

#[tauri::command]
pub(crate) async fn reset_nsm(nsm: State<'_, Mutex<NetworkStreamManager>>) -> Result<(), ()> {
    let mut nsm = nsm.lock().await;
    _ = nsm.reset().await;
    Ok(())
}
