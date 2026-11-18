use std::time::Duration;

use tokio::net::TcpStream;
use tokio::task::JoinHandle;
use tokio::time::Instant;

/// Finds which default port a server typed without a port accepts connections on.
///
/// Used only when no port was typed and no SRV record names one. A server embedded in a mod
/// often cannot bind TCP 443 and moves its API onto 28280, the QUIC default; a bare name for
/// such a server must still connect.
pub struct DefaultPortProbe {
    ports: Vec<u16>,
    budget: Duration,
}

impl DefaultPortProbe {
    /// The candidates, in order of preference: the HTTPS default, then the QUIC default.
    pub const STANDARD_PORTS: [u16; 2] = [443, 28280];
    pub const BUDGET: Duration = Duration::from_millis(1500);

    pub fn new(ports: Vec<u16>, budget: Duration) -> Self {
        Self { ports, budget }
    }

    pub fn standard() -> Self {
        Self::new(Self::STANDARD_PORTS.to_vec(), Self::BUDGET)
    }

    /// The first candidate, in list order, that accepts a TCP connection to `host`. `None`
    /// when none does within the budget.
    ///
    /// Every candidate is dialled at once under one deadline. The answer is ready as soon as
    /// every port ahead of the first success has failed, so a reachable first port never
    /// waits on a later one.
    pub async fn pick(&self, host: &str) -> Option<u16> {
        let deadline = Instant::now() + self.budget;
        let mut attempts: Vec<(u16, JoinHandle<bool>)> = self
            .ports
            .iter()
            .map(|&port| (port, tokio::spawn(Self::accepts(host.to_string(), port, deadline))))
            .collect();

        let mut picked = None;
        for (port, attempt) in attempts.iter_mut() {
            if attempt.await.unwrap_or(false) {
                picked = Some(*port);
                break;
            }
        }
        // The ones still dialling lost to an earlier port.
        for (_, attempt) in &attempts {
            attempt.abort();
        }
        picked
    }

    async fn accepts(host: String, port: u16, deadline: Instant) -> bool {
        matches!(
            tokio::time::timeout_at(deadline, TcpStream::connect((host.as_str(), port))).await,
            Ok(Ok(_))
        )
    }
}
