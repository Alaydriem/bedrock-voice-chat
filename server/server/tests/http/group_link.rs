use crate::harness::TestServer;
use common::structs::control::{
    GroupLinkAction, GroupLinkDrain, GroupLinkEvent, GroupLinkEventKind, LinkedChannelView,
};

const TOKEN: &str = "test-mc-token";

#[tokio::test]
async fn group_link_rejects_missing_token() {
    let env = TestServer::start().await.unwrap();
    let client = env.noauth_client().unwrap();
    let body = GroupLinkAction::CreateLinked {
        creator: "Steve".into(),
        name: "Raid".into(),
        members_only: true,
    };

    let resp = client
        .post(format!("{}/api/group-link", env.base_url))
        .json(&body)
        .send()
        .await
        .unwrap();
    assert_ne!(resp.status(), 200);

    let events = client
        .get(format!("{}/api/group-link/events?after=0", env.base_url))
        .send()
        .await
        .unwrap();
    assert_ne!(events.status(), 200);
}

#[tokio::test]
async fn create_linked_returns_id_and_sets_members_only() {
    let env = TestServer::start().await.unwrap();
    let client = env.noauth_client().unwrap();
    let body = GroupLinkAction::CreateLinked {
        creator: "Steve".into(),
        name: "Raid".into(),
        members_only: true,
    };

    let resp = client
        .post(format!("{}/api/group-link", env.base_url))
        .header("Authorization", format!("Bearer {TOKEN}"))
        .json(&body)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let id: Option<String> = resp.json().await.unwrap();
    let id = id.expect("CreateLinked returns the channel id");

    let channels: Vec<LinkedChannelView> = client
        .get(format!("{}/api/group-link/channels", env.base_url))
        .header("Authorization", format!("Bearer {TOKEN}"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let linked = channels
        .iter()
        .find(|channel| channel.id == id)
        .expect("the new channel is listed");
    assert_eq!(linked.name, "Raid");
    assert!(linked.members_only);
}

// The Kotlin bridge builds this JSON by hand; GroupLinkClientTest pins the same literal.
#[test]
fn the_kotlin_create_linked_literal_is_a_create_linked_action() {
    let action: GroupLinkAction = serde_json::from_str(
        r#"{"CreateLinked":{"creator":"Steve","name":"Raid","members_only":true}}"#,
    )
    .expect("the bridge's JSON decodes");

    assert_eq!(
        action,
        GroupLinkAction::CreateLinked {
            creator: "Steve".into(),
            name: "Raid".into(),
            members_only: true,
        }
    );
}

// The bridge reads the drain with field names it declares itself; GroupLinkClientTest parses
// this literal.
#[test]
fn a_drain_encodes_to_the_shape_the_kotlin_bridge_reads() {
    let drain = GroupLinkDrain {
        events: vec![GroupLinkEvent {
            seq: 7,
            kind: GroupLinkEventKind::Join,
            channel: "ABCD".into(),
            channel_name: Some("Raid".into()),
            player: Some("Alex".into()),
            members_only: true,
        }],
        reset: true,
        epoch: 4503599627370495,
    };

    assert_eq!(
        serde_json::to_string(&drain).unwrap(),
        r#"{"events":[{"seq":7,"kind":"Join","channel":"ABCD","channel_name":"Raid","player":"Alex","members_only":true}],"reset":true,"epoch":4503599627370495}"#
    );
}
