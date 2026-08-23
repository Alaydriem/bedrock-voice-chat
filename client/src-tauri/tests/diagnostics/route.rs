use bvc_client_lib::websocket::{ListenerKind, RejectReason, WebSocketRoute};

const KEY: &str = "s3cret-key";

#[test]
fn root_path_routes_to_the_command_protocol() {
    assert_eq!(WebSocketRoute::resolve("/", ListenerKind::External, KEY), Ok(WebSocketRoute::Command));
    assert_eq!(WebSocketRoute::resolve("", ListenerKind::External, KEY), Ok(WebSocketRoute::Command));
}

#[test]
fn the_command_path_does_not_require_a_query_key() {
    // Authentication on the command protocol is per message and must stay that way, or every
    // existing integration breaks at the handshake.
    assert_eq!(WebSocketRoute::resolve("/", ListenerKind::External, KEY), Ok(WebSocketRoute::Command));
}

#[test]
fn metrics_path_routes_to_the_push_stream() {
    assert_eq!(
        WebSocketRoute::resolve("/metrics?key=s3cret-key", ListenerKind::External, KEY),
        Ok(WebSocketRoute::Metrics)
    );
}

#[test]
fn metrics_path_tolerates_a_trailing_slash() {
    assert_eq!(
        WebSocketRoute::resolve("/metrics/?key=s3cret-key", ListenerKind::External, KEY),
        Ok(WebSocketRoute::Metrics)
    );
}

#[test]
fn metrics_path_without_a_key_is_rejected_when_a_key_is_configured() {
    // A push-only stream has no inbound message to carry a key, so the upgrade is refused rather
    // than accepted and left silent — silence is indistinguishable from a healthy quiet link.
    assert_eq!(
        WebSocketRoute::resolve("/metrics", ListenerKind::External, KEY),
        Err(RejectReason::MissingKey)
    );
}

#[test]
fn metrics_path_with_a_wrong_key_is_rejected() {
    assert_eq!(
        WebSocketRoute::resolve("/metrics?key=wrong", ListenerKind::External, KEY),
        Err(RejectReason::InvalidKey)
    );
}

#[test]
fn metrics_path_is_open_when_no_key_is_configured() {
    // Matches what the command path already does with an empty key rather than inventing a
    // stricter rule for one endpoint.
    assert_eq!(
        WebSocketRoute::resolve("/metrics", ListenerKind::External, ""),
        Ok(WebSocketRoute::Metrics)
    );
}

#[test]
fn a_percent_encoded_key_still_matches() {
    assert_eq!(
        WebSocketRoute::resolve("/metrics?key=a%20b", ListenerKind::External, "a b"),
        Ok(WebSocketRoute::Metrics)
    );
}

#[test]
fn other_query_parameters_do_not_hide_the_key() {
    assert_eq!(
        WebSocketRoute::resolve("/metrics?foo=1&key=s3cret-key&bar=2", ListenerKind::External, KEY),
        Ok(WebSocketRoute::Metrics)
    );
}

#[test]
fn an_empty_key_value_does_not_authenticate() {
    assert_eq!(
        WebSocketRoute::resolve("/metrics?key=", ListenerKind::External, KEY),
        Err(RejectReason::InvalidKey)
    );
}

#[test]
fn a_lookalike_parameter_is_not_mistaken_for_the_key() {
    assert_eq!(
        WebSocketRoute::resolve("/metrics?keyx=s3cret-key", ListenerKind::External, KEY),
        Err(RejectReason::MissingKey)
    );
}

#[test]
fn a_wrong_first_key_is_not_rescued_by_a_later_correct_one() {
    // First occurrence wins. Accepting a later duplicate would let a caller smuggle a valid key
    // past anything that inspected only the first.
    assert_eq!(
        WebSocketRoute::resolve("/metrics?key=wrong&key=s3cret-key", ListenerKind::External, KEY),
        Err(RejectReason::InvalidKey)
    );
}

#[test]
fn an_unrecognised_path_reaches_the_command_protocol() {
    // The previous `accept_async` never inspected the path, so an integration on `/ws` or using an
    // absolute-form request target upgraded fine. Rejecting those would break third-party clients
    // silently at the handshake, and the command protocol authenticates per message regardless of
    // the path it arrived on.
    for uri in ["/ws", "/bvc", "//", "///", "http://127.0.0.1:9595/", "/metrics/extra"] {
        assert_eq!(
            WebSocketRoute::resolve(uri, ListenerKind::External, KEY),
            Ok(WebSocketRoute::Command),
            "{uri} must reach the command protocol"
        );
    }
}

#[test]
fn malformed_percent_escapes_do_not_panic() {
    // The decoder indexes into a byte slice, so a truncated or invalid escape must not be able to
    // slice out of bounds.
    for uri in [
        "/metrics?key=%",
        "/metrics?key=%2",
        "/metrics?key=%zz",
        "/metrics?key=abc%",
        "?",
        "/metrics?",
    ] {
        let _ = WebSocketRoute::resolve(uri, ListenerKind::External, KEY);
    }
}

#[test]
fn a_near_miss_of_the_metrics_path_does_not_reach_the_push_stream() {
    // `/metrics` is the one path with upgrade-time authentication, so anything that is not exactly
    // it must not be admitted to the push stream — an unauthenticated metrics frame carries speaker
    // names, the server hostname and device names.
    for uri in ["/metricsx", "/Metrics", "/metrics/extra", "/api/metrics"] {
        assert_ne!(
            WebSocketRoute::resolve(uri, ListenerKind::External, KEY),
            Ok(WebSocketRoute::Metrics),
            "{uri} must not be routed to the push stream"
        );
    }
}
