// What a session needs to open.
//
// Strings rather than parsed types because this crosses an FFI boundary
// unchanged: parsing here means one error surface instead of two, and the caller
// gets a message naming the field it got wrong.
#[derive(Debug, Clone)]
pub struct SessionConfig {
    // Where `node.key` lives. Created on first use and reused after, because the
    // far side's `peer` block names the key this directory holds.
    pub node_dir: String,
    pub peerlink: String,
    pub worlds: Vec<String>,
    pub relay_url: Option<String>,
    pub inbox_capacity: usize,
}
