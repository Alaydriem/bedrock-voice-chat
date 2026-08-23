use common::net::NetTimeouts;
use common::structs::reachability::{
    AnsweredVia, EndpointReachability, ReachabilityOutcome, ServerReachability, VoiceChoice,
};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::Duration;

fn addr(port: u16) -> SocketAddr {
    SocketAddr::new(IpAddr::V4(Ipv4Addr::new(203, 0, 113, 1)), port)
}

fn quic(rtt_micros: u32) -> Vec<EndpointReachability> {
    vec![EndpointReachability::new(
        addr(443),
        ReachabilityOutcome::Answered {
            via: AnsweredVia::VersionNegotiation,
            rtt_micros,
        },
        None,
    )]
}

fn quic_silent() -> Vec<EndpointReachability> {
    vec![EndpointReachability::new(
        addr(443),
        ReachabilityOutcome::Silent,
        None,
    )]
}

fn ws(rtt_micros: u32) -> Vec<EndpointReachability> {
    vec![EndpointReachability::new(
        addr(443),
        ReachabilityOutcome::Answered {
            via: AnsweredVia::VoiceWebSocket,
            rtt_micros,
        },
        None,
    )]
}

fn https() -> Vec<EndpointReachability> {
    vec![EndpointReachability::new(
        addr(443),
        ReachabilityOutcome::Answered {
            via: AnsweredVia::Https,
            rtt_micros: 8_000,
        },
        None,
    )]
}

fn report(
    quic: Vec<EndpointReachability>,
    ws: Vec<EndpointReachability>,
) -> ServerReachability {
    ServerReachability::new("choice.test".to_string(), quic, https(), ws)
}

fn margin() -> Duration {
    NetTimeouts::WEBSOCKET_PREFERENCE_MARGIN
}

#[test]
fn quic_is_chosen_when_it_is_the_only_transport_that_answered() {
    assert_eq!(report(quic(16_000), Vec::new()).voice_choice(margin()), VoiceChoice::Quic);
}

#[test]
fn the_fallback_is_chosen_when_quic_did_not_answer() {
    assert_eq!(
        report(quic_silent(), ws(116_000)).voice_choice(margin()),
        VoiceChoice::WebSocket
    );
}

#[test]
fn quic_is_chosen_when_it_is_the_faster_of_the_two() {
    assert_eq!(
        report(quic(16_000), ws(116_000)).voice_choice(margin()),
        VoiceChoice::Quic
    );
}

// Distance inflates both transports, and inflates the fallback more: it costs a TCP
// handshake and a TLS handshake where QUIC costs one round trip. A player who is merely
// far away must therefore keep QUIC, which is why the preference needs a margin rather
// than a comparison.
#[test]
fn a_fallback_that_is_merely_faster_does_not_displace_quic() {
    assert_eq!(
        report(quic(900_000), ws(300_000)).voice_choice(margin()),
        VoiceChoice::Quic
    );
}

// A QUIC path that answers seconds later than a TCP one to the same host is not distant,
// it is degraded — losing Initials and backing off. Voice carried over it is worse than
// voice carried over the fallback.
#[test]
fn a_fallback_that_beats_quic_by_the_whole_margin_is_chosen() {
    assert_eq!(
        report(quic(3_000_000), ws(200_000)).voice_choice(margin()),
        VoiceChoice::WebSocket
    );
}

#[test]
fn the_margin_is_exclusive_at_its_own_boundary() {
    let exactly = 200_000 + margin().as_micros() as u32;

    assert_eq!(
        report(quic(exactly), ws(200_000)).voice_choice(margin()),
        VoiceChoice::Quic
    );
    assert_eq!(
        report(quic(exactly + 1), ws(200_000)).voice_choice(margin()),
        VoiceChoice::WebSocket
    );
}

// A server that never advertised the transport has an empty leg, which is not the same
// as one measured and found silent. Neither leaves anywhere to fall back to.
#[test]
fn an_unmeasured_fallback_leaves_quic_as_the_only_choice() {
    assert_eq!(
        report(quic(900_000), Vec::new()).voice_choice(margin()),
        VoiceChoice::Quic
    );
}

#[test]
fn nothing_answering_on_either_transport_is_no_choice() {
    assert_eq!(
        report(quic_silent(), Vec::new()).voice_choice(margin()),
        VoiceChoice::None
    );
    assert_eq!(
        report(quic_silent(), ws(0)).voice_choice(Duration::ZERO),
        VoiceChoice::WebSocket
    );
}
