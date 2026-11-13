use super::SrvAnswer;

/// Chooses the record a client connects with.
pub struct SrvRecordChoice;

impl SrvRecordChoice {
    // Only a record that names the queried host with a usable port is a candidate: a client
    // never follows an SRV target to a different name, and a `.` target means no service.
    // Lowest priority wins, then highest weight, then lowest port.
    pub fn pick(queried_host: &str, answers: &[SrvAnswer]) -> Option<SrvAnswer> {
        let wanted = Self::normalize(queried_host);

        answers
            .iter()
            .filter(|answer| {
                answer.port != 0
                    && !answer.target.trim_end_matches('.').is_empty()
                    && Self::normalize(&answer.target) == wanted
            })
            .min_by_key(|answer| {
                (
                    answer.priority,
                    std::cmp::Reverse(answer.weight),
                    answer.port,
                )
            })
            .cloned()
    }

    fn normalize(name: &str) -> String {
        name.trim_end_matches('.').to_ascii_lowercase()
    }
}
