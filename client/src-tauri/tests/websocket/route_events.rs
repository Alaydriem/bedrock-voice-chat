use bvc_client_lib::websocket::{ListenerKind, RejectReason, WebSocketRoute};

const TOKEN: &str = "process-token";

#[test]
fn the_internal_listener_accepts_events_with_the_process_token() {
    assert_eq!(
        WebSocketRoute::resolve("/events?key=process-token", ListenerKind::Internal, TOKEN),
        Ok(WebSocketRoute::Events)
    );
}

#[test]
fn the_internal_listener_refuses_events_without_the_token() {
    assert_eq!(
        WebSocketRoute::resolve("/events", ListenerKind::Internal, TOKEN),
        Err(RejectReason::MissingKey)
    );
    assert_eq!(
        WebSocketRoute::resolve("/events?key=guess", ListenerKind::Internal, TOKEN),
        Err(RejectReason::InvalidKey)
    );
}

// The internal listener exists to push meter frames to this app's own window. The command
// protocol reaches mute, recording and world connection, and nothing in the webview needs it
// over a socket — it uses `invoke`. So the internal listener serves one route and refuses the
// rest, rather than putting the command surface behind a second credential.
#[test]
fn the_internal_listener_serves_nothing_but_events() {
    for uri in ["/", "/metrics", "/anything"] {
        assert_eq!(
            WebSocketRoute::resolve(uri, ListenerKind::Internal, TOKEN),
            Err(RejectReason::InvalidRoute),
            "internal listener must refuse {uri}"
        );
    }
}

#[test]
fn the_external_listener_still_resolves_the_command_protocol_and_metrics() {
    assert_eq!(
        WebSocketRoute::resolve("/", ListenerKind::External, "userkey"),
        Ok(WebSocketRoute::Command)
    );
    assert_eq!(
        WebSocketRoute::resolve("/ws", ListenerKind::External, "userkey"),
        Ok(WebSocketRoute::Command)
    );
    assert_eq!(
        WebSocketRoute::resolve("/metrics?key=userkey", ListenerKind::External, "userkey"),
        Ok(WebSocketRoute::Metrics)
    );
}

// A third-party integration may subscribe to the richer stream with the key it already has.
#[test]
fn the_external_listener_accepts_events_with_the_user_key() {
    assert_eq!(
        WebSocketRoute::resolve("/events?key=userkey", ListenerKind::External, "userkey"),
        Ok(WebSocketRoute::Events)
    );
    assert_eq!(
        WebSocketRoute::resolve("/events?key=wrong", ListenerKind::External, "userkey"),
        Err(RejectReason::InvalidKey)
    );
}

// The process token must not be reachable from outside. It is longer-lived than a user key and
// is never shown to anybody, so a leak has no rotation story.
#[test]
fn the_internal_token_is_not_accepted_on_the_external_listener() {
    assert_eq!(
        WebSocketRoute::resolve("/events?key=process-token", ListenerKind::External, "userkey"),
        Err(RejectReason::InvalidKey)
    );
}
