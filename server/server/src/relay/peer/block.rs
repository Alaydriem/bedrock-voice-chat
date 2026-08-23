// A `peers` block, rendered for an operator to paste inside the `server` block of
// config.hcl.
//
// `peers`, not `peer`: hcl-rs names the resulting map after the block identifier
// verbatim, and the field this has to reach is `Server::peers`. A `peer` block is
// valid HCL that lands in a key nothing reads, so the server goes on reporting
// "peering is not configured" with the grant sitting in the file.
//
// Nothing in this codebase writes config.hcl. The file is the operator's, it
// carries their comments and their formatting, and a tool that rewrites it owns
// a merge problem it cannot see. Printing the block leaves the edit where the
// person making it can read the surrounding file — which is also why the
// surrounding `server` block is described rather than rendered: emitting it would
// invite a second one into a file that already has it.
pub struct PeerBlock;

impl PeerBlock {
    pub fn render(label: &str, peerlink: &str) -> String {
        format!(
            "peers \"{}\" {{\n  peerlink = \"{}\"\n}}\n",
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
