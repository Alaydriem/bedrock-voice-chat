// A `peer` block, rendered for an operator to paste into config.hcl.
//
// Nothing in this codebase writes config.hcl. The file is the operator's, it
// carries their comments and their formatting, and a tool that rewrites it owns
// a merge problem it cannot see. Printing the block leaves the edit where the
// person making it can read the surrounding file.
pub struct PeerBlock;

impl PeerBlock {
    pub fn render(label: &str, peerlink: &str) -> String {
        format!(
            "peer \"{}\" {{\n  peerlink = \"{}\"\n}}\n",
            Self::escape(label),
            Self::escape(peerlink)
        )
    }

    // A peer link is base32 and can never need this. A label is whatever the
    // operator names their bridge, and one carrying a quote would close the
    // string early in a file they have already saved.
    fn escape(value: &str) -> String {
        value.replace('\\', "\\\\").replace('"', "\\\"")
    }
}
