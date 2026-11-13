// What a play-name check-in recorded.
#[derive(Debug, Clone)]
pub struct PlayCheckIn {
    // The bare label.
    pub name: String,
    pub nonce: String,
    // True when the address or the port differs from the published ones.
    pub endpoint_changed: bool,
    // True when an earlier check-in is too recent: the nonce is that check-in's and no
    // probe should start.
    pub throttled: bool,
}
