use common::structs::AnalyticsEventData;
use common::net::ConnectCandidate;

mod attempt_result;

pub(crate) use attempt_result::AttemptResult;

/// Every candidate the walk tried, in order, with what became of it.
///
/// A connect failure currently reaches the operator as a single error code, which cannot
/// separate "no UDP left this network" from "the server refused us" from "one family was
/// unroutable". The walk already knows all three; this carries them off the device.
pub(crate) struct ConnectOutcome {
    attempts: Vec<(ConnectCandidate, AttemptResult)>,
}

impl ConnectOutcome {
    pub(crate) fn new() -> Self {
        Self {
            attempts: Vec::new(),
        }
    }

    pub(crate) fn record(&mut self, candidate: ConnectCandidate, result: AttemptResult) {
        self.attempts.push((candidate, result));
    }

    /// Flattened rather than nested: PostHog filters on scalar properties, and a nested array
    /// would have to be unpacked at query time on every question worth asking.
    pub(crate) fn properties(&self, server: &str) -> AnalyticsEventData {
        let connected = self
            .attempts
            .iter()
            .find(|(_, result)| *result == AttemptResult::Connected);

        let mut data = AnalyticsEventData::new()
            .insert("server", server.to_string())
            .insert("attempts", self.attempts.len() as u64)
            .insert("connected", connected.is_some())
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

        if let Some((candidate, _)) = connected {
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
