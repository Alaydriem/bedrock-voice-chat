use common::net::ConnectCandidate;
use common::structs::AnalyticsEventData;
use common::structs::metrics::TransportKind;

mod attempt_result;
mod fallback_reason;

pub use attempt_result::AttemptResult;
pub use fallback_reason::FallbackReason;

/// One connect attempt, from the probe's choice to the transport that carried the session.
///
/// A connect failure currently reaches the operator as a single error code, which cannot
/// separate "no UDP left this network" from "the server refused us" from "one family was
/// unroutable". The walk already knows all three; this carries them off the device.
///
/// The verdict is the transport, not the walk. A player whose every QUIC candidate timed out
/// and who is talking over the WebSocket is connected, and an outcome that reported the walk
/// alone recorded that session as a failure — undercounting precisely the players the fallback
/// exists for.
pub struct ConnectOutcome {
    attempts: Vec<(ConnectCandidate, AttemptResult)>,
    transport: Option<TransportKind>,
    fallback: FallbackReason,
}

impl ConnectOutcome {
    pub fn new() -> Self {
        Self {
            attempts: Vec::new(),
            transport: None,
            fallback: FallbackReason::None,
        }
    }

    pub fn record(&mut self, candidate: ConnectCandidate, result: AttemptResult) {
        self.attempts.push((candidate, result));
    }

    /// Names the transport the session is running on. Unset means nothing carried.
    pub fn carried(&mut self, transport: TransportKind) {
        self.transport = Some(transport);
    }

    pub fn fell_back(&mut self, reason: FallbackReason) {
        self.fallback = reason;
    }

    /// Flattened rather than nested: PostHog filters on scalar properties, and a nested array
    /// would have to be unpacked at query time on every question worth asking.
    pub fn properties(&self, server: &str) -> AnalyticsEventData {
        let winner = self
            .attempts
            .iter()
            .find(|(_, result)| *result == AttemptResult::Connected);

        let mut data = AnalyticsEventData::new()
            .insert("server", server.to_string())
            .insert(
                "transport",
                self.transport.map(|t| t.as_str()).unwrap_or("none"),
            )
            .insert("fallback", self.fallback.as_str())
            .insert("connected", self.transport.is_some())
            .insert("attempts", self.attempts.len() as u64)
            .insert(
                "timed_out",
                self.attempts
                    .iter()
                    .filter(|(_, r)| *r == AttemptResult::TimedOut)
                    .count() as u64,
            )
            .insert(
                "rejected",
                self.attempts
                    .iter()
                    .filter(|(_, r)| *r == AttemptResult::Rejected)
                    .count() as u64,
            );

        // Named only where a QUIC candidate won. A WebSocket session has no winning port or
        // family, and borrowing the last candidate it tried would read as one.
        if let Some((candidate, _)) = winner {
            data = data
                .insert("winning_port", candidate.port())
                .insert("winning_family", format!("{:?}", candidate.family()));
        }

        // The ordered walk as one string — "443/Ipv6=timed_out,443/Ipv4=connected" — so a
        // single property answers which ports and families a network actually permits.
        let walk = self
            .attempts
            .iter()
            .map(|(candidate, result)| {
                format!(
                    "{}/{:?}={}",
                    candidate.port(),
                    candidate.family(),
                    result.as_str()
                )
            })
            .collect::<Vec<_>>()
            .join(",");

        data.insert("walk", walk)
    }
}

impl Default for ConnectOutcome {
    fn default() -> Self {
        Self::new()
    }
}
