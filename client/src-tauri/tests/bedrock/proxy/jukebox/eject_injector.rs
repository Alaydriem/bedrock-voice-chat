use bvc_client_lib::bedrock::JukeboxEjectInjector;
use common::structs::packet::{BedrockEvent, BedrockEventDirection, BedrockEventPacket};
use common::structs::game::Coordinate;

fn make_announcement(event_id: &str, world_uuid: &str) -> BedrockEventPacket {
    BedrockEventPacket::with_direction(
        BedrockEvent::JukeboxEjectAnnouncement {
            event_id: event_id.to_string(),
            block_pos: Coordinate {
                x: 1.0,
                y: 2.0,
                z: 3.0,
            },
        },
        world_uuid.to_string(),
        BedrockEventDirection::ClientBound,
    )
}

#[test]
fn enqueues_first_arrival() {
    let injector = JukeboxEjectInjector::new();
    let rx = injector.receiver();
    injector.handle_packet(&make_announcement("e1", "world-x"));
    let job = rx.try_recv().expect("first arrival should enqueue");
    assert_eq!(job.event_id, "e1");
    assert_eq!(job.world_uuid, "world-x");
}

#[test]
fn dedups_repeat_event_id() {
    let injector = JukeboxEjectInjector::new();
    let rx = injector.receiver();
    injector.handle_packet(&make_announcement("e1", "world-x"));
    injector.handle_packet(&make_announcement("e1", "world-x"));
    assert!(rx.try_recv().is_ok());
    assert!(rx.try_recv().is_err());
}

#[test]
fn ignores_non_client_bound_direction() {
    let injector = JukeboxEjectInjector::new();
    let rx = injector.receiver();
    let mut pkt = make_announcement("e1", "world-x");
    pkt.direction = BedrockEventDirection::ServerBound;
    injector.handle_packet(&pkt);
    assert!(rx.try_recv().is_err());
}

#[test]
fn ignores_non_announcement_variants() {
    let injector = JukeboxEjectInjector::new();
    let rx = injector.receiver();
    let pkt = BedrockEventPacket::with_direction(
        BedrockEvent::JukeboxEject {
            event_id: "e1".to_string(),
            player_xuid: "p1".to_string(),
        },
        "world-x".to_string(),
        BedrockEventDirection::ClientBound,
    );
    injector.handle_packet(&pkt);
    assert!(rx.try_recv().is_err());
}
