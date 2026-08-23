use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr};
use std::time::{Duration, Instant};

use tokio::net::UdpSocket;

use super::{ProbeInitialPacket, RouteProbe};
use crate::net::NetTimeouts;
use crate::structs::reachability::{AnsweredVia, ReachabilityOutcome};

pub struct NegotiationProbe;

impl NegotiationProbe {
    // A single lost probe would otherwise burn the whole budget, and the probe
    // runs on the connect path where that delay is visible to a player.
    const RETRANSMIT_AFTER: Duration = Duration::from_millis(250);

    pub async fn probe(dest: SocketAddr) -> ReachabilityOutcome {
        if !RouteProbe::is_routable(dest) {
            return ReachabilityOutcome::NoRoute;
        }

        match Self::exchange(dest).await {
            Some(rtt) => ReachabilityOutcome::Answered {
                via: AnsweredVia::VersionNegotiation,
                rtt_micros: rtt.as_micros().min(u32::MAX as u128) as u32,
            },
            None => ReachabilityOutcome::Silent,
        }
    }

    async fn exchange(dest: SocketAddr) -> Option<Duration> {
        let bind: SocketAddr = match dest {
            SocketAddr::V4(_) => (Ipv4Addr::UNSPECIFIED, 0).into(),
            SocketAddr::V6(_) => (Ipv6Addr::UNSPECIFIED, 0).into(),
        };

        let socket = UdpSocket::bind(bind).await.ok()?;
        socket.connect(dest).await.ok()?;

        let packet = ProbeInitialPacket::new();
        let started = Instant::now();
        socket.send(packet.datagram()).await.ok()?;

        let mut buf = vec![0u8; 2048];
        let mut retransmitted = false;

        loop {
            let remaining = NetTimeouts::NEGOTIATION.checked_sub(started.elapsed())?;
            let wait = if retransmitted {
                remaining
            } else {
                remaining.min(Self::RETRANSMIT_AFTER)
            };

            match tokio::time::timeout(wait, socket.recv(&mut buf)).await {
                Ok(Ok(len)) if packet.accepts_reply(&buf[..len]) => return Some(started.elapsed()),
                // A packet that is not our reply leaves the budget running rather
                // than ending the attempt.
                Ok(_) => continue,
                Err(_) if !retransmitted => {
                    retransmitted = true;
                    socket.send(packet.datagram()).await.ok()?;
                }
                Err(_) => return None,
            }
        }
    }
}
