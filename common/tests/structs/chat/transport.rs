use common::structs::bedrock::AddonMode;
use common::structs::chat::ChatTransport;

// The declaration is the whole input. A net world's addon owns chat, so the line goes to the
// BVC server, which holds that addon's channel — never to the proxy, which discards it.
#[test]
fn a_net_world_routes_to_the_server() {
    assert_eq!(
        ChatTransport::for_mode(Some(AddonMode::Net)),
        ChatTransport::Server
    );
}

// No-net means this proxy IS the world's chat implementation.
#[test]
fn a_no_net_world_routes_to_the_proxy() {
    assert_eq!(
        ChatTransport::for_mode(Some(AddonMode::NoNet)),
        ChatTransport::ProxyInjection
    );
}

// No proxy session at all: the app is a plain client of the BVC server, and the server
// carries chat. The proxy injector's queue is drained only by a live per-connection session,
// so routing there would leave the line with no consumer.
#[test]
fn no_session_routes_to_the_server() {
    assert_eq!(ChatTransport::for_mode(None), ChatTransport::Server);
}

// Reachability is deliberately absent from the signature. A mode says which component owns
// chat, and that does not stop being true because the server restarted — gating on liveness
// is what silently dropped a typed line instead of delivering or refusing it.
#[test]
fn the_same_mode_always_yields_the_same_transport() {
    for _ in 0..3 {
        assert_eq!(
            ChatTransport::for_mode(Some(AddonMode::Net)),
            ChatTransport::Server
        );
    }
}
