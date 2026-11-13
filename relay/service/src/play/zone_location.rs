// Where a queried name sits relative to the play zone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ZoneLocation {
    // The zone's own name.
    Apex,
    // Below the zone. Holds everything left of the zone, which is a single label for
    // every name the registry publishes.
    Host(String),
    // `_bvc._tcp.<label>`, where the SRV record of a label lives. Holds the label.
    Service(String),
    // `_tcp.<label>`, which exists only because the service name sits below it. Holds
    // the label.
    EmptyNonTerminal(String),
    // Not in the zone.
    Outside,
}
