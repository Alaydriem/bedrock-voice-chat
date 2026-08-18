use crate::analytics::AnalyticsService;
use crate::{NetworkStreamManager, structs::app_state::AppState};
use common::consts::version::PROTOCOL_VERSION;
use common::net::{CandidatePlan, NetTimeouts, ReachabilityPlanner};
use common::response::api::config::ProtocolCompatibility;
use common::response::LoginResponse;
use common::structs::reachability::ServerReachability;
use log::{error, info, warn};
use std::sync::Arc;
use tauri::Manager;
use tauri::State;
use tauri::async_runtime::Mutex;

#[tauri::command]
pub(crate) async fn stop_network_stream(
    app: tauri::AppHandle,
    network_stream: State<'_, Mutex<NetworkStreamManager>>,
    analytics: State<'_, Arc<AnalyticsService>>,
) -> Result<(), ()> {
    let mut network_stream = network_stream.lock().await;
    _ = network_stream.stop().await;
    analytics.clear_connected_server();
    analytics.clear_player();
    // Back to permissive: a disconnected client must not go on refusing under the
    // policy of the server it just left.
    app.state::<std::sync::Arc<crate::chat::ChatPolicy>>()
        .set_enabled(true);
    crate::audio::AudioActionsManager::new(app)
        .set_recording_allowed(true)
        .await;
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
    // A second handle: the invalidation below consumes the first, and the credential probe on
    // the failure path runs after it.
    let api_for_probe = api.as_ref().ok().cloned();

    let advertised = match api {
        Ok(api) => match api.get_config().await {
            Ok(config) => Some((
                config.quic_ports,
                config.quic_port,
                config.recording.enabled,
                config.voice_websocket,
                config.chat.enabled,
            )),
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

    // A config fetch that failed leaves recording permitted, the same answer an
    // unasked server gives, and leaves the fallback transport worth trying. The two
    // wrong answers are not symmetric: assuming no fallback strands a player whose
    // UDP is blocked, while assuming one costs a TLS handshake that gets refused.
    let (advertised_ports, advertised_scalar, recording_enabled, voice_websocket, chat_enabled) =
        advertised.unwrap_or((Vec::new(), 0, true, true, true));

    crate::audio::AudioActionsManager::new(app.clone())
        .set_recording_allowed(recording_enabled)
        .await;
    if !recording_enabled {
        info!("{} does not permit recording", server);
    }

    app.state::<std::sync::Arc<crate::chat::ChatPolicy>>()
        .set_enabled(chat_enabled);
    if !chat_enabled {
        info!("{} has chat sync disabled", server);
    }

    let request = match ReachabilityPlanner::plan(
        &server,
        &advertised_ports,
        advertised_scalar,
        Some(&data.quic_connect_string),
        voice_websocket,
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
    let choice = report.voice_choice(NetTimeouts::WEBSOCKET_PREFERENCE_MARGIN);

    info!(
        "QUIC candidates for {} ({:?}): {:?}",
        server_fqdn,
        report.preference(),
        plan.candidates()
            .iter()
            .map(|c| c.dial())
            .collect::<Vec<_>>()
    );

    // Both measurements, beside the choice they produced. A transport picked against
    // expectation is answerable from one line rather than from the timing of the lines
    // after it.
    info!(
        "Voice transport for {}: {:?} (QUIC {:?} us, fallback {:?} us)",
        server_fqdn,
        choice,
        report.best_rtt_micros(),
        report.fallback_rtt_micros()
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
    // The error is inspected and rendered before any further await: it is a
    // `Box<dyn Error>`, which is not Send, and holding one across an await would
    // make this command's future non-Send.
    let outcome = match network_stream
        .restart(
            server_fqdn.clone(),
            server.clone(),
            plan,
            choice,
            voice_websocket,
            identity,
            data.certificate_ca,
            data.certificate,
            data.certificate_key,
        )
        .await
    {
        Ok(()) => Ok(()),
        Err(e) => {
            let certificate = e
                .downcast_ref::<crate::network::ConnectFailure>()
                .map(|failure| failure.is_certificate())
                .unwrap_or(false);
            Err((certificate, format!("{:?}", e)))
        }
    };

    match outcome {
        Ok(()) => {
            info!("Now streaming {}", server);
            analytics.set_connected_server(Some(server.clone()));
            analytics.set_player(&gamertag);
        }
        Err((certificate, detail)) => {
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

            if certificate {
                drop(network_stream);

                // A rejected handshake is evidence, not proof, and a keyring entry destroyed in
                // error costs the player their saved server. Ask the one endpoint that answers
                // the question directly.
                let verdict = match api_for_probe {
                    Some(api) => api.verify_credentials().await,
                    None => crate::api::CredentialVerdict::Inconclusive,
                };

                match verdict {
                    crate::api::CredentialVerdict::Rejected => {
                        error!(
                            "{} no longer accepts this device's certificate; signing out",
                            server
                        );

                        let mut state = state.lock().await;
                        let keyring = app.state::<Mutex<crate::keyring::KeyringService>>();
                        let mut kr = keyring.lock().await;
                        if let Err(e) = crate::auth::SessionService::new(app.clone())
                            .forget_current_server(&mut state, &mut kr, &analytics)
                            .await
                        {
                            error!("Could not clear credentials for {}: {}", server, e);
                        }

                        return Err(format!("CERT_INVALID: {}", detail));
                    }
                    // The credentials work. The voice listener's certificate is the server
                    // operator's to fix, and taking a working sign-in away would not help.
                    crate::api::CredentialVerdict::Valid => {
                        error!(
                            "{} accepts this device's certificate over HTTPS but its voice transport does not",
                            server
                        );
                        return Err(format!("SERVER_CERT: {}", detail));
                    }
                    // Nothing was established. Credentials are never destroyed on this.
                    crate::api::CredentialVerdict::Inconclusive => {
                        warn!(
                            "could not confirm whether {} still accepts this device's certificate; keeping it",
                            server
                        );
                    }
                }
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
    // Whether the server advertised the WebSocket voice transport, from the
    // `/api/config` the caller has already read. Supplied rather than fetched again
    // for the same reason `quic_ports` is.
    voice_websocket: bool,
    state: State<'_, Mutex<AppState>>,
) -> Result<ServerReachability, String> {
    let request =
        ReachabilityPlanner::plan(&server, &quic_ports, quic_port, None, voice_websocket)
            .await
            .map_err(|e| e.to_string())?;

    let reachability = {
        let state = state.lock().await;
        state.reachability()
    };

    Ok(reachability.evaluate(&request).await)
}

// The address field's question, which is narrower than the preflight's: whether voice can
// reach this address at all. Which transport carries it is settled at connect time, from the
// complete report, so waiting out the QUIC budget here buys the screen nothing and costs it
// seconds — the WebSocket probe answers in milliseconds and the QUIC walk has a full
// handshake budget per endpoint.
//
// The report this returns may therefore be incomplete, and its verdict means "a path exists"
// rather than "this is the path". Never hand it to `CandidatePlan::build`. The measurement
// finishes in the background and caches the whole thing, so the connect that follows still
// reads a complete report from this same instance.
#[tauri::command]
pub(crate) async fn probe_voice_path(
    server: String,
    quic_ports: Vec<u32>,
    quic_port: u32,
    voice_websocket: bool,
    state: State<'_, Mutex<AppState>>,
) -> Result<ServerReachability, String> {
    let request =
        ReachabilityPlanner::plan(&server, &quic_ports, quic_port, None, voice_websocket)
            .await
            .map_err(|e| e.to_string())?;

    let reachability = {
        let state = state.lock().await;
        state.reachability()
    };

    Ok(reachability.evaluate_any_voice_path(&request).await)
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
