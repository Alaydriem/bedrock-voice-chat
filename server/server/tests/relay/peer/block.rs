use bvc_server_lib::relay::PeerBlock;

// The block is pasted into config.hcl by hand, so it has to parse as written —
// including the label, which is a quoted HCL block label rather than an
// identifier.
#[test]
fn renders_a_block_that_parses_as_hcl() {
    let rendered = PeerBlock::render("svc-bridge", "bvcpeerabc123");

    assert_eq!(
        rendered,
        "peer \"svc-bridge\" {\n  peerlink = \"bvcpeerabc123\"\n}\n"
    );
}

// A label carrying a quote would otherwise close the string and produce a block
// that fails to parse in a file the operator has already saved.
#[test]
fn escapes_a_quote_in_the_label() {
    let rendered = PeerBlock::render("a\"b", "bvcpeerabc123");

    assert!(
        rendered.starts_with("peer \"a\\\"b\" {"),
        "unescaped label: {rendered}"
    );
}
