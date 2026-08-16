use std::collections::HashMap;

use bvc_server_lib::runtime::position_updater::PositionUpdater;
use bvc_server_lib::stream::quic::WebhookReceiver;
use bvc_server_lib::stream::quic::connection::{ConnectionRegistry, RoutedPacket};
use common::Game;
use common::PlayerEnum;
use common::structs::packet::{MAX_DATAGRAM_SIZE, QuicNetworkPacket, QuicNetworkPacketData};
use common::traits::player_data::PlayerData;
use tokio::sync::mpsc;

use crate::harness::PositionFixture;

// Voice clients are a small minority of a realm's population; that gap is
// exactly what addressing positions per-player saves.
const VOICE_CLIENTS: usize = 4;

// Either side of the 14-player ceiling that used to break serialization, the
// population actually observed in production, and well beyond it.
const POPULATIONS: [usize; 10] = [1, 4, 13, 14, 15, 26, 30, 50, 100, 250];

/// Each connected client must receive its own position, whatever the roster
/// size. This is the only entry a client reads: it anchors the listener for
/// spatial audio, and losing it strands everyone at world origin.
#[tokio::test]
async fn every_connected_client_receives_its_own_position() {
    for population in POPULATIONS {
        for world in PositionFixture::WORLD_FORMS {
            let roster = PositionFixture::roster(population, world);
            let listeners = pick_listeners(&roster);
            let delivered = deliver(&roster, &listeners).await;

            for name in &listeners {
                let received = delivered
                    .get(name)
                    .unwrap_or_else(|| panic!("population {population}: {name} was not served"));

                let own = received.get(name).unwrap_or_else(|| {
                    panic!(
                        "population {population}, world {world:?}: {name} never received \
                         its own position"
                    )
                });

                let expected = roster
                    .iter()
                    .find(|p| p.get_name() == name)
                    .expect("listener is drawn from the roster");

                assert_eq!(
                    own.get_position(),
                    expected.get_position(),
                    "population {population}: {name} got the wrong coordinates"
                );
                assert_eq!(
                    own.world_uuid(),
                    expected.world_uuid(),
                    "population {population}: {name} lost world identity"
                );
            }
        }
    }
}

/// A client must never be sent another player's coordinates. Emitter positions
/// ride the audio frame, so the roster was pure waste on the wire -- and a
/// client that receives it also learns where everyone on the realm is standing.
#[tokio::test]
async fn no_client_receives_another_players_position() {
    const POPULATION: usize = 100;

    let roster = PositionFixture::roster(POPULATION, PositionFixture::WORLD_FORMS[0]);
    let listeners = pick_listeners(&roster);
    let delivered = deliver(&roster, &listeners).await;

    for name in &listeners {
        let received = &delivered[name];

        assert_eq!(
            received.len(),
            1,
            "{name} received {} players; a client only ever needs its own",
            received.len()
        );

        for other in received.keys() {
            assert_eq!(other, name, "{name} was sent {other}'s position");
        }
    }
}

/// Players with no voice connection must generate no traffic at all. With a
/// realm of 100 and four clients on voice, exactly four packets should leave
/// the server per tick.
#[tokio::test]
async fn players_without_a_voice_connection_produce_no_traffic() {
    const POPULATION: usize = 100;

    let roster = PositionFixture::roster(POPULATION, PositionFixture::WORLD_FORMS[0]);
    let listeners = pick_listeners(&roster);

    let (registry, mut receivers) = registry_with(&listeners);

    for packet in emit_packets(&roster).await {
        registry.send_positions_to_owners(&packet);
    }

    let total: usize = receivers
        .iter_mut()
        .map(|(_, rx)| {
            let mut n = 0;
            while rx.try_recv().is_ok() {
                n += 1;
            }
            n
        })
        .sum();

    assert_eq!(
        total, VOICE_CLIENTS,
        "expected one datagram per connected client, got {total} for a roster of {POPULATION}"
    );
}

/// A client can be connected to voice while absent from the roster -- it joined
/// the channel but not the game yet, or the position feed has not caught up. It
/// must simply receive nothing, never a substitute or another player's entry.
#[tokio::test]
async fn client_absent_from_the_roster_receives_nothing() {
    let roster = PositionFixture::roster(25, PositionFixture::WORLD_FORMS[0]);

    let mut listeners = pick_listeners(&roster);
    let absent = String::from("NotInTheRoster");
    listeners.push(absent.clone());

    let delivered = deliver(&roster, &listeners).await;

    assert!(
        delivered[&absent].is_empty(),
        "a client missing from the roster was sent {} record(s)",
        delivered[&absent].len()
    );

    for name in listeners.iter().filter(|n| **n != absent) {
        assert!(
            delivered[name].contains_key(name),
            "{name} did not receive its own position"
        );
    }
}

/// Nothing the position path emits may exceed the datagram bound, at any roster
/// size, name length or world identifier format.
#[tokio::test]
async fn no_emitted_datagram_exceeds_the_maximum() {
    for population in POPULATIONS {
        for world in PositionFixture::WORLD_FORMS {
            let roster = PositionFixture::roster(population, world);

            for packet in emit_packets(&roster).await {
                let bytes = packet
                    .to_datagram()
                    .unwrap_or_else(|e| panic!("population {population}, world {world:?}: {e}"));

                assert!(
                    bytes.len() <= MAX_DATAGRAM_SIZE,
                    "population {population}, world {world:?}: emitted {} bytes, max {MAX_DATAGRAM_SIZE}",
                    bytes.len()
                );
            }
        }
    }
}

/// A roster spanning two worlds cannot be compacted, since the hoisted
/// identifier is only lossless when every player agrees. Each client must still
/// receive its own world unchanged.
#[tokio::test]
async fn mixed_world_roster_preserves_each_player_world() {
    let roster = PositionFixture::mixed_world_roster(40);
    let listeners = pick_listeners(&roster);
    let delivered = deliver(&roster, &listeners).await;

    for name in &listeners {
        let expected = roster
            .iter()
            .find(|p| p.get_name() == name)
            .expect("listener is drawn from the roster");

        let own = delivered[name]
            .get(name)
            .unwrap_or_else(|| panic!("{name} never received its own position"));

        assert_eq!(
            own.world_uuid(),
            expected.world_uuid(),
            "{name}: world identity crossed over"
        );
    }
}

/// The realistic end-to-end shape: a full realm arrives as the JSON the BDS mod
/// POSTs to `/api/position`, and each voice client ends up anchored on itself.
#[tokio::test]
async fn hundred_player_mod_payload_anchors_every_client() {
    const POPULATION: usize = 100;

    for world in PositionFixture::WORLD_FORMS {
        let payload = PositionFixture::mod_payload_json(POPULATION, world);

        let collection: common::GameDataCollection = serde_json::from_str(&payload)
            .unwrap_or_else(|e| panic!("world {world:?}: mod payload did not deserialize: {e}"));

        assert_eq!(collection.players.len(), POPULATION);

        let listeners = pick_listeners(&collection.players);
        let delivered = deliver(&collection.players, &listeners).await;

        for name in &listeners {
            assert!(
                delivered[name].contains_key(name),
                "world {world:?}: {name} never received its own position"
            );
        }
    }
}

// Spreads the listeners across the roster so the selection is not clustered at
// one end of the packing.
fn pick_listeners(roster: &[PlayerEnum]) -> Vec<String> {
    let stride = (roster.len() / VOICE_CLIENTS).max(1);

    roster
        .iter()
        .step_by(stride)
        .take(VOICE_CLIENTS)
        .map(|p| p.get_name().to_string())
        .collect()
}

fn registry_with(
    listeners: &[String],
) -> (
    ConnectionRegistry,
    Vec<(String, mpsc::Receiver<RoutedPacket>)>,
) {
    let registry = ConnectionRegistry::new();
    let mut receivers = Vec::with_capacity(listeners.len());

    for (i, name) in listeners.iter().enumerate() {
        let (tx, rx) = mpsc::channel(4096);
        registry.register(i as u64, Game::Minecraft.membership_key(&name), tx);
        receivers.push((name.clone(), rx));
    }

    (registry, receivers)
}

// Runs the real production path: PositionUpdater packs the roster and publishes
// through the webhook channel exactly as the /api/position route does.
async fn emit_packets(players: &[PlayerEnum]) -> Vec<QuicNetworkPacket> {
    let (webhook_tx, mut webhook_rx) = mpsc::unbounded_channel();
    let receiver = WebhookReceiver::new(webhook_tx);

    PositionUpdater::broadcast_positions(players.to_vec(), &receiver).await;
    drop(receiver);

    let mut packets = Vec::new();
    while let Some(packet) = webhook_rx.recv().await {
        packets.push(packet);
    }
    packets
}

// Routes the emitted packets to the listeners and reassembles what each one
// actually received off the wire, keyed by recipient.
async fn deliver(
    roster: &[PlayerEnum],
    listeners: &[String],
) -> HashMap<String, HashMap<String, PlayerEnum>> {
    let (registry, receivers) = registry_with(listeners);

    for packet in emit_packets(roster).await {
        registry.send_positions_to_owners(&packet);
    }

    let mut delivered = HashMap::new();

    for (name, mut rx) in receivers {
        let mut merged: HashMap<String, PlayerEnum> = HashMap::new();

        while let Ok(RoutedPacket::Serialized(bytes)) = rx.try_recv() {
            // from_datagram enforces the size bound on the receiving side, so a
            // decode failure here is itself the regression.
            let packet = QuicNetworkPacket::from_datagram(&bytes)
                .expect("client received an undecodable datagram");

            let QuicNetworkPacketData::PlayerData(data) = packet.data else {
                continue;
            };

            for player in data.players {
                merged.insert(player.get_name().to_string(), player);
            }
        }

        delivered.insert(name, merged);
    }

    delivered
}
