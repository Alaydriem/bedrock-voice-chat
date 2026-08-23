use bvc_server_lib::config::Server;
use bvc_server_lib::relay::PeerBlock;

// The block is pasted into config.hcl by hand, so it has to parse as written and
// land in the field the runtime reads — `Server::peers`, which is what
// `GrantTable::from_config` is given. Comparing the rendered string alone let a
// block that parses as valid HCL, into a key nothing reads, pass as correct: the
// server reported peering as unconfigured with the grant sitting in the file.
#[test]
fn the_rendered_block_deserializes_into_the_field_the_runtime_reads() {
    let rendered = PeerBlock::render("svc-bridge", "bvcpeerabc123");

    let value: serde_json::Value = hcl::from_str(&rendered).expect("parse as hcl");
    let server: Server = serde_json::from_value(value).expect("deserialize into server config");

    let peer = server
        .peers
        .get("svc-bridge")
        .expect("the rendered label keys the grant");
    assert_eq!(peer.peerlink, "bvcpeerabc123");
}

// A label carrying a quote would otherwise close the string and produce a block
// that fails to parse in a file the operator has already saved.
#[test]
fn escapes_a_quote_in_the_label() {
    let rendered = PeerBlock::render("a\"b", "bvcpeerabc123");

    assert!(
        rendered.starts_with("peers \"a\\\"b\" {"),
        "unescaped label: {rendered}"
    );
}
