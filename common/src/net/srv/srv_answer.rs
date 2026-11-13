/// One SRV record from an answer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SrvAnswer {
    pub priority: u16,
    pub weight: u16,
    pub port: u16,
    pub target: String,
    pub ttl: u32,
}
