use std::net::IpAddr;

// What the play zone holds for one label: the proven address, and the TCP port the
// check-in probe reached it on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlayZoneEntry {
    pub address: IpAddr,
    pub port: Option<u16>,
}
