use common::curia;
use common::structs::relay::enroll::PlayPreflightOutcome;

use super::PlayAddressProblem;

/// Says what a preflight found, in sentences an operator can act on.
pub struct PlayPreflightReport;

impl PlayPreflightReport {
    // The registry saw this server at an address that is not public, and none was declared.
    pub const OBSERVED_NOT_PUBLIC: &'static str = "Play preflight failed: the registry saw this server at a private address. The registry and this server share a private network, or the registry is behind NAT that hides the server's address. Set server.tls.play.address to this server's public IP.";

    // An address the operator declared is not public.
    pub const DECLARED_NOT_PUBLIC: &'static str = "Play preflight failed: the address is not a public IP address. Players on the internet cannot reach it. Remove server.tls.play.address, or set it to this server's public IP.";

    pub const TIMED_OUT: &'static str = "Play preflight failed: the registry could not reach this server. On a home network, forward tcp_port and udp_port on the router to this machine, and allow both in the firewall. If the router's WAN address is in 100.64.0.0/10, the network uses CGNAT. If this server's public IP is not address, set server.tls.play.address to it.";

    pub const REFUSED: &'static str = "Play preflight failed: nothing accepted the connection on tcp_port. Check that the router forwards it to this machine and that no other program uses it. If the router's WAN address is in 100.64.0.0/10, the network uses CGNAT. If this server's public IP is not address, set server.tls.play.address to it.";

    /// The message for an address the registry refused, by whether the operator declared it.
    pub fn not_public_message(declared: bool) -> &'static str {
        if declared {
            Self::DECLARED_NOT_PUBLIC
        } else {
            Self::OBSERVED_NOT_PUBLIC
        }
    }

    /// `declared` is whether the request named an address.
    pub fn log_outcome(outcome: &PlayPreflightOutcome, quic_port: u16, declared: bool) {
        match outcome {
            PlayPreflightOutcome::Reachable { address, port } => {
                curia::info!("Play preflight passed", { "address": address.clone(), "port": *port });
            }
            PlayPreflightOutcome::AddressNotPublic { address } => {
                curia::error!(Self::not_public_message(declared), { "address": address.clone() });
            }
            PlayPreflightOutcome::AddressCgnat { address } => {
                curia::error!(
                    "Play preflight failed: the address is behind carrier-grade NAT (CGNAT), so port forwarding cannot work. Host the server on a machine with a public IP, or use an entitled name for LAN play.",
                    { "address": address.clone() }
                );
            }
            PlayPreflightOutcome::TimedOut { address, port } => {
                curia::error!(
                    Self::TIMED_OUT,
                    { "address": address.clone(), "tcp_port": *port, "udp_port": quic_port }
                );
            }
            PlayPreflightOutcome::Refused { address, port } => {
                curia::error!(
                    Self::REFUSED,
                    { "address": address.clone(), "tcp_port": *port }
                );
            }
            PlayPreflightOutcome::WrongService { address, port } => {
                curia::error!(
                    "Play preflight failed: another service answered on tcp_port. The router forwards it to a different machine or program.",
                    { "address": address.clone(), "tcp_port": *port }
                );
            }
            PlayPreflightOutcome::RateLimited { retry_after_secs } => {
                curia::info!("Play preflight deferred by the registry", { "retry_in": *retry_after_secs });
            }
        }
    }

    pub fn log_problem(problem: &PlayAddressProblem) {
        match problem {
            PlayAddressProblem::ConfiguredNotPublic { address, class } => {
                curia::error!(
                    "Play mode cannot start: server.tls.play.address is not a public address. Players on the internet cannot reach it. Remove the setting to use the public address the registry sees, or set it to this server's public IP.",
                    { "address": address.to_string(), "class": format!("{class:?}") }
                );
            }
            PlayAddressProblem::BehindCgnat { interface } => {
                curia::warn!(
                    "This server's default route is in 100.64.0.0/10. A home network behind carrier-grade NAT (CGNAT) cannot take a port forward, while a hosting platform or VPN in that range can. Play mode continues to the registry's preflight, which decides. If the preflight fails, set server.tls.play.address to this server's public IP.",
                    { "interface": interface.to_string() }
                );
            }
        }
    }
}
