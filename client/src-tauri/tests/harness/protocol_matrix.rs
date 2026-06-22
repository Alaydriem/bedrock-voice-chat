use common::bedrock_protocol::version::ProtocolVersion;

pub struct ProtocolMatrix;

impl ProtocolMatrix {
    /// Protocol coverage is the last two generated versions — derived, never hardcoded.
    pub fn last_two() -> Vec<ProtocolVersion> {
        let all = ProtocolVersion::GENERATED_ALL;
        all[all.len() - 2..].to_vec()
    }
}
