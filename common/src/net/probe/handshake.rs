use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr};
use std::sync::Arc;
use std::time::{Duration, Instant};

use s2n_quic::Client;
use s2n_quic::client::Connect;

use super::{ProbeCertVerifier, ProbeTlsProvider, RouteProbe};
use crate::structs::reachability::{AnsweredVia, ObservedCertificate, ReachabilityOutcome};

pub struct HandshakeProbe;

impl HandshakeProbe {
    pub const BUDGET: Duration = Duration::from_secs(3);

    pub async fn probe(
        dest: SocketAddr,
        server_name: &str,
    ) -> (ReachabilityOutcome, Option<ObservedCertificate>) {
        if !RouteProbe::is_routable(dest) {
            return (ReachabilityOutcome::NoRoute, None);
        }

        let (verifier, observed) = ProbeCertVerifier::new();
        let bind: SocketAddr = match dest {
            SocketAddr::V4(_) => (Ipv4Addr::UNSPECIFIED, 0).into(),
            SocketAddr::V6(_) => (Ipv6Addr::UNSPECIFIED, 0).into(),
        };

        let client = match Self::build_client(bind, verifier) {
            Ok(client) => client,
            Err(_) => return (ReachabilityOutcome::Silent, None),
        };

        let connect = Connect::new(dest).with_server_name(server_name.to_string());
        let started = Instant::now();
        let result = tokio::time::timeout(Self::BUDGET, client.connect(connect)).await;
        let rtt_micros = started.elapsed().as_micros().min(u32::MAX as u128) as u32;

        let certificate = observed
            .lock()
            .ok()
            .and_then(|slot| slot.clone())
            .and_then(|der| ObservedCertificate::from_der(der.as_ref()));

        // A captured certificate is the proof. Without one, a failure could equally
        // be a local error, so the outcome stays Silent — which orders this family
        // last but still attempts it.
        match result {
            Ok(Ok(_connection)) => (
                ReachabilityOutcome::Answered {
                    via: AnsweredVia::Handshake,
                    rtt_micros,
                },
                certificate,
            ),
            Ok(Err(_)) if certificate.is_some() => (
                ReachabilityOutcome::Answered {
                    via: AnsweredVia::TlsRejection,
                    rtt_micros,
                },
                certificate,
            ),
            _ => (ReachabilityOutcome::Silent, None),
        }
    }

    // No datagram provider: the probe never sends application data, it only needs
    // the handshake to get far enough for the server to present a certificate.
    fn build_client(
        bind: SocketAddr,
        verifier: Arc<ProbeCertVerifier>,
    ) -> Result<Client, anyhow::Error> {
        let client = Client::builder()
            .with_tls(ProbeTlsProvider::new(verifier))?
            .with_io(bind)?
            .start()?;

        Ok(client)
    }
}
