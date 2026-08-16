use common::net::{NegotiationProbe, NetTimeouts, ProbeInitialPacket};
use common::structs::reachability::{AnsweredVia, ReachabilityOutcome};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use tokio::net::UdpSocket;

// Stands in for a QUIC server: reads one probe, reflects its Source Connection ID
// back inside a Version Negotiation packet. Building the reply from the received
// bytes is what makes this exercise the real accept rule.
pub async fn spawn_negotiating_server() -> SocketAddr {
    let socket = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    let addr = socket.local_addr().unwrap();

    tokio::spawn(async move {
        let mut buf = vec![0u8; 2048];
        while let Ok((len, from)) = socket.recv_from(&mut buf).await {
            if len < 23 {
                continue;
            }

            let dcid = buf[6..14].to_vec();
            let scid = buf[15..23].to_vec();

            let mut reply = vec![0x80u8, 0, 0, 0, 0];
            reply.push(scid.len() as u8);
            reply.extend_from_slice(&scid);
            reply.push(dcid.len() as u8);
            reply.extend_from_slice(&dcid);
            reply.extend_from_slice(&1u32.to_be_bytes());

            let _ = socket.send_to(&reply, from).await;
        }
    });

    addr
}

/// A bound port that reads and never replies, which is what a blackholed UDP path looks
/// like: the negotiation probe waits out its budget and the handshake probe then waits out
/// its own.
pub async fn spawn_silent_server() -> SocketAddr {
    let socket = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    let addr = socket.local_addr().unwrap();

    tokio::spawn(async move {
        let mut buf = vec![0u8; 2048];
        while socket.recv_from(&mut buf).await.is_ok() {}
    });

    addr
}

#[tokio::test]
async fn a_negotiating_server_answers_with_a_measured_round_trip() {
    let addr = spawn_negotiating_server().await;

    match NegotiationProbe::probe(addr).await {
        ReachabilityOutcome::Answered { via, rtt_micros } => {
            assert_eq!(via, AnsweredVia::VersionNegotiation);
            assert!(
                (rtt_micros as u128) <= NetTimeouts::NEGOTIATION.as_micros(),
                "measured {rtt_micros}us, budget is {}us",
                NetTimeouts::NEGOTIATION.as_micros()
            );
        }
        other => panic!("expected Answered, got {other:?}"),
    }
}

#[tokio::test]
async fn a_bound_but_silent_port_is_silent_not_unreachable() {
    let addr = spawn_silent_server().await;

    assert_eq!(
        NegotiationProbe::probe(addr).await,
        ReachabilityOutcome::Silent
    );
}

// A closed port on loopback cannot be distinguished from an unusable route, and
// both must resolve to an outcome rather than an error.
#[tokio::test]
async fn a_loopback_destination_never_reports_as_answered() {
    let dest = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 1);

    assert!(!NegotiationProbe::probe(dest).await.answered());
}

#[tokio::test]
async fn the_probe_sends_exactly_the_padded_initial() {
    let socket = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    let addr = socket.local_addr().unwrap();

    let observer = tokio::spawn(async move {
        let mut buf = vec![0u8; 4096];
        let (len, _) = socket.recv_from(&mut buf).await.unwrap();
        len
    });

    let _ = NegotiationProbe::probe(addr).await;

    assert_eq!(observer.await.unwrap(), ProbeInitialPacket::DATAGRAM_LEN);
}
