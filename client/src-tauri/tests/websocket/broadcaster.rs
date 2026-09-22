use bvc_client_lib::websocket::WebSocketBroadcaster;
use common::structs::players::PlayerSource;
use common::structs::voice::{VoiceMember, VoiceRoster};

fn roster(names: &[&str]) -> VoiceRoster {
    VoiceRoster {
        own: "minecraft:Alaydriem".to_string(),
        members: names
            .iter()
            .map(|name| VoiceMember {
                name: (*name).to_string(),
                sources: vec![PlayerSource::Proximity],
                gamerpic: None,
            })
            .collect(),
    }
}

/// A quiet session produces no roster changes for hours, so a subscriber that arrives between
/// changes has to be told the current roster rather than wait for the next one. This is the
/// same requirement `health` and `levels` already carry.
#[test]
fn a_subscriber_is_seeded_with_the_current_roster() {
    let broadcaster = WebSocketBroadcaster::for_test();
    broadcaster.broadcast_roster(roster(&["minecraft:VoxelWren"]));

    let seeded = broadcaster
        .seed_frames()
        .into_iter()
        .any(|frame| frame.contains("\"roster\"") && frame.contains("minecraft:VoxelWren"));

    assert!(
        seeded,
        "the seed must carry the roster a late subscriber missed"
    );
}

#[test]
fn the_retained_roster_is_the_most_recent_one() {
    let broadcaster = WebSocketBroadcaster::for_test();
    broadcaster.broadcast_roster(roster(&["minecraft:VoxelWren"]));
    broadcaster.broadcast_roster(roster(&["minecraft:PistonPete"]));

    let latest = broadcaster.latest_roster();
    assert_eq!(latest.members.len(), 1);
    assert_eq!(latest.members[0].name, "minecraft:PistonPete");
}

/// Retention is the reason these channels are `watch` rather than plain broadcasts, and it went
/// unexercised: nothing holds a receiver for them, and `watch::Sender::send` refuses to store a
/// value when the receiver count is zero.
#[test]
fn the_seed_reports_the_health_that_was_last_broadcast() {
    let broadcaster = WebSocketBroadcaster::for_test();
    broadcaster.broadcast_health(common::structs::network::ConnectionHealth::Connected);

    assert!(
        matches!(
            broadcaster.latest_health(),
            common::structs::network::ConnectionHealth::Connected
        ),
        "a connected link must not seed a subscriber as disconnected"
    );
}

#[test]
fn the_seed_reports_the_levels_that_were_last_broadcast() {
    use common::structs::audio::{LevelSnapshot, ParticipantLevel};

    let broadcaster = WebSocketBroadcaster::for_test();
    broadcaster.broadcast_levels(LevelSnapshot {
        own: ParticipantLevel {
            speaking: true,
            loudness: 5,
        },
        peers: Default::default(),
    });

    let seeded = broadcaster
        .seed_frames()
        .into_iter()
        .any(|frame| frame.contains("\"levels\"") && frame.contains("\"loudness\":5"));

    assert!(seeded, "the seed must carry the levels a late subscriber missed");
}
