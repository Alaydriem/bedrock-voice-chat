use serde::{Deserialize, Serialize};

// One declared peer.
//
// Peering is declared, never discovered: a node absent from this map is refused
// at connect. `peerlink` is the value the far side printed — it carries the
// peer's public key and the paths to reach it, so it is the only field an
// operator has to supply.
//
// `worlds` is an optional narrowing filter, not a declaration. A peer says which
// worlds it hosts during the handshake, because that is a fact about its own
// deployment. Leaving this empty accepts what the peer says; naming worlds
// restricts the link to their intersection. It never grants a world the peer did
// not declare.
//
// Held as strings because this type also drives the JSON schema and the
// generated Kotlin config mirror, neither of which can describe an iroh key.
// `GrantTable::from_config` parses and reports failures per block.
#[derive(Serialize, Deserialize, Debug, Clone, Default, schemars::JsonSchema)]
pub struct PeerConfig {
    pub peerlink: String,

    #[serde(default)]
    pub worlds: Vec<String>,

    #[serde(default = "PeerConfig::default_capabilities")]
    pub capabilities: Vec<String>,
}

impl PeerConfig {
    // An omitted list is the ordinary case — an operator adding a voice bridge
    // wants it to carry voice. Reaching the audio library stays opt-in, so
    // `query_audio` and `serve_audio` are never granted by omission.
    //
    // An explicitly empty list is left empty: that is a deliberate statement, and
    // overriding it would make the field inoperable.
    pub fn default_capabilities() -> Vec<String> {
        vec!["carry_speakers".to_string()]
    }
}
