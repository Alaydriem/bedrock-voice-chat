use bvc_client_lib::bedrock::{BedrockTargetService, ResolvedAddress, SavedProxyEntry};
use common::response::ApiConfigBedrockServer;
use common::structs::bedrock::{AddonMode, RealmEntry};
use websocket_types::ConnectTargetKind;

fn saved(id: &str, name: &str, host: &str, port: u16) -> SavedProxyEntry {
    SavedProxyEntry {
        id: id.to_string(),
        name: name.to_string(),
        host: host.to_string(),
        port,
        protocol_version: None,
        addon_mode: None,
    }
}

fn advertised(name: &str, host: &str, port: u16) -> ApiConfigBedrockServer {
    ApiConfigBedrockServer {
        name: name.to_string(),
        host: host.to_string(),
        port,
        protocol_version: None,
        addon_mode: AddonMode::NoNet,
    }
}

fn realm(id: u64, name: &str) -> RealmEntry {
    RealmEntry {
        id,
        name: name.to_string(),
        motd: String::new(),
        state: "OPEN".to_string(),
        owner_uuid: String::new(),
    }
}

// Every source mints its id the same way the webview does, so a controller's picker and the
// app's own list name the same world identically.
#[test]
fn mints_a_namespaced_id_for_every_source() {
    let service = BedrockTargetService::new(
        vec![saved("V1StGXR8", "Survival", "10.0.0.5", 19132)],
        vec![advertised("Main", "play.example.com", 19132)],
        vec![realm(1234567, "My Realm")],
    );

    let ids: Vec<String> = service.targets().into_iter().map(|t| t.id).collect();

    assert!(ids.contains(&"saved:V1StGXR8".to_string()));
    assert!(ids.contains(&"server:play.example.com:19132".to_string()));
    assert!(ids.contains(&"realm:1234567".to_string()));
}

// The operator's curated list and the user's saved list overlap in practice: a user saves the
// server their operator already advertises. Listing both would show every such world twice,
// and the saved entry wins because it carries the name and protocol override the user chose.
#[test]
fn a_saved_entry_displaces_an_advertised_one_at_the_same_address() {
    let service = BedrockTargetService::new(
        vec![saved("V1StGXR8", "My name for it", "play.example.com", 19132)],
        vec![advertised("Operator name", "play.example.com", 19132)],
        vec![],
    );

    let targets = service.targets();

    assert_eq!(targets.len(), 1);
    assert_eq!(targets[0].name, "My name for it");
    assert_eq!(targets[0].id, "saved:V1StGXR8");
}

#[test]
fn keeps_an_advertised_entry_that_no_saved_entry_shadows() {
    let service = BedrockTargetService::new(
        vec![saved("V1StGXR8", "Survival", "10.0.0.5", 19132)],
        vec![advertised("Main", "play.example.com", 19132)],
        vec![],
    );

    assert_eq!(service.targets().len(), 2);
}

// The wire form is what a controller picks from, and it must never require an address.
#[test]
fn the_wire_form_carries_no_address() {
    let service = BedrockTargetService::new(vec![], vec![], vec![realm(1234567, "My Realm")]);
    let targets = service.targets();

    assert_eq!(targets[0].kind, ConnectTargetKind::Realm);
    assert_eq!(targets[0].name, "My Realm");
}

#[test]
fn resolves_a_realm_id_to_its_numeric_id() {
    let service = BedrockTargetService::new(vec![], vec![], vec![realm(1234567, "My Realm")]);

    let resolved = service.resolve("realm:1234567").expect("realm should resolve");

    assert_eq!(resolved.address, ResolvedAddress::Realm { realm_id: 1234567 });
}

#[test]
fn resolves_an_advertised_id_to_its_address() {
    let service = BedrockTargetService::new(
        vec![],
        vec![advertised("Main", "play.example.com", 19132)],
        vec![],
    );

    let resolved = service
        .resolve("server:play.example.com:19132")
        .expect("advertised entry should resolve");

    assert_eq!(
        resolved.address,
        ResolvedAddress::Proxy {
            host: "play.example.com".to_string(),
            port: 19132,
            protocol_version: None,
        }
    );
}

#[test]
fn refuses_an_id_no_source_recognises() {
    let service = BedrockTargetService::new(vec![], vec![], vec![realm(1234567, "My Realm")]);

    assert!(service.resolve("1234567").is_none());
}

// A session started from the app is named by looking its address up in this same list, so the
// state frame and a `targets` listing cannot disagree about what a world is called.
#[test]
fn finds_a_running_session_by_the_address_it_was_started_with() {
    let service = BedrockTargetService::new(
        vec![saved("V1StGXR8", "Survival", "10.0.0.5", 19132)],
        vec![],
        vec![],
    );

    let resolved = service
        .resolve_by_address("10.0.0.5", 19132)
        .expect("a saved entry should be found by its address");

    assert_eq!(resolved.id, "saved:V1StGXR8");
    assert_eq!(resolved.to_active().name, "Survival");
}

#[test]
fn reports_no_match_for_an_address_nothing_names() {
    let service = BedrockTargetService::new(vec![], vec![], vec![]);

    assert!(service.resolve_by_address("10.0.0.5", 19132).is_none());
}
