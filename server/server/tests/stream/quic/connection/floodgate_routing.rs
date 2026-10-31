//! A Floodgate player is heard, and hears, as themselves.
//!
//! The router resolves a speaker by the identity their connection authenticated with — the
//! Common Name of the certificate the server minted for their record — in the position cache
//! the game server's roster filled. If the roster names the player differently from their
//! record, the lookup misses and the player is signed in but silent. Every row of the gate
//! runs through the same calls production makes: canonicalize, register, record aliases,
//! cache, route.

use std::sync::Arc;

use bvc_server_lib::stream::quic::connection::{ConnectionRegistry, RoutedPacket};
use bvc_server_lib::stream::quic::CertificateCommonName;
use common::PlayerEnum;
use moka::future::Cache;
use tokio::sync::mpsc;

use crate::harness::{GameServerReport, GateRows, RoutingFixture};

const RANGE: f32 = 30.0;
const WHISPER: f32 = 0.0;
const LISTENER: &str = "Listener";

type PlayerCache = Arc<Cache<String, Arc<PlayerEnum>>>;

struct RoutingGate;

impl RoutingGate {
    // The identity a player's client would connect with: the CN of their record's
    // certificate. None when the server never registered them.
    async fn identity_of(report: &GameServerReport, gamertag: &str) -> Option<String> {
        let record = report.record(gamertag).await?;
        let (_, pem) = x509_parser::pem::parse_x509_pem(record.certificate.as_bytes()).ok()?;
        CertificateCommonName::from_der(&pem.contents)
    }

    // Routes one frame from `speaker` and returns what each named connection received.
    async fn route(
        cache: &PlayerCache,
        connections: &[&str],
        speaker: &str,
    ) -> Result<Vec<Option<common::structs::packet::QuicNetworkPacket>>, String> {
        let reg = ConnectionRegistry::new();
        let mut receivers: Vec<mpsc::Receiver<RoutedPacket>> = Vec::new();
        for (index, identity) in connections.iter().enumerate() {
            let (tx, rx) = mpsc::channel(8);
            reg.try_register(index as u64 + 1, (*identity).into(), format!("fp-{index}"), tx)
                .map_err(|_| format!("{identity} refused admission"))?;
            receivers.push(rx);
        }

        let Some(position) = cache.get(speaker).await else {
            return Err(format!("speaker {speaker} unresolved"));
        };
        let position = (*position).clone();
        let packet = RoutingFixture::audio_packet(position.clone(), speaker);
        reg.route_audio_frame(&packet, Some(&position), cache, RANGE, WHISPER)
            .await;

        let mut delivered = Vec::new();
        for rx in receivers.iter_mut() {
            delivered.push(RoutingFixture::delivered_envelope(rx).await);
        }
        Ok(delivered)
    }

    fn sender_of(packet: &Option<common::structs::packet::QuicNetworkPacket>) -> Option<String> {
        packet
            .as_ref()
            .and_then(|p| p.sender_identity().map(|id| id.to_string()))
    }

    // Ingests `roster`, then checks that `gamertag` is heard by the Listener under its own
    // identity and hears the Listener. With a decoy, also checks the decoy is heard under
    // the decoy's identity and never stands in for the gamertag.
    async fn check(
        report: &GameServerReport,
        label: &str,
        roster: Vec<PlayerEnum>,
        gamertag: &str,
        decoy: Option<&str>,
    ) -> Vec<String> {
        let mut failures = Vec::new();
        let roster = report.ingest(roster).await;
        let cache = RoutingFixture::player_cache(&roster).await;

        let Some(player) = Self::identity_of(report, gamertag).await else {
            return vec![format!("{label}: no record for {gamertag:?}")];
        };
        let Some(listener) = Self::identity_of(report, LISTENER).await else {
            return vec![format!("{label}: no record for the listener")];
        };
        let decoy = match decoy {
            Some(name) => match Self::identity_of(report, name).await {
                Some(identity) => Some(identity),
                None => return vec![format!("{label}: no record for decoy {name:?}")],
            },
            None => None,
        };

        let mut connections = vec![player.as_str(), listener.as_str()];
        if let Some(ref d) = decoy {
            connections.push(d.as_str());
        }

        match Self::route(&cache, &connections, &player).await {
            Ok(delivered) => {
                if Self::sender_of(&delivered[1]).as_deref() != Some(player.as_str()) {
                    failures.push(format!(
                        "{label} speaks: listener got {:?}",
                        Self::sender_of(&delivered[1])
                    ));
                }
            }
            Err(e) => failures.push(format!("{label} speaks: {e}")),
        }

        match Self::route(&cache, &connections, &listener).await {
            Ok(delivered) => {
                if Self::sender_of(&delivered[0]).as_deref() != Some(listener.as_str()) {
                    failures.push(format!(
                        "{label} listens: got {:?}",
                        Self::sender_of(&delivered[0])
                    ));
                }
            }
            Err(e) => failures.push(format!("{label} listens: {e}")),
        }

        if let Some(ref d) = decoy {
            match Self::route(&cache, &connections, d).await {
                Ok(delivered) => {
                    if Self::sender_of(&delivered[1]).as_deref() != Some(d.as_str()) {
                        failures.push(format!(
                            "{label} decoy speaks: listener got {:?}",
                            Self::sender_of(&delivered[1])
                        ));
                    }
                }
                Err(e) => failures.push(format!("{label} decoy speaks: {e}")),
            }
        }

        failures
    }

    fn listener() -> PlayerEnum {
        GameServerReport::player_at(LISTENER, None, None, 1.0)
    }
}

#[tokio::test]
async fn every_gamertag_in_the_gate_is_heard_and_hears() {
    let mut failures = Vec::new();

    for row in GateRows::ALL {
        for (wire_label, wire) in [("beta27", row.beta27), ("new", row.new)] {
            let report = GameServerReport::new().await;
            let mut roster = vec![
                GameServerReport::player_at(wire.0, wire.1, Some(row.uuid), 0.0),
                RoutingGate::listener(),
            ];
            let decoy = match row.id {
                "G10" => {
                    roster.push(GameServerReport::player_at(
                        "TTV_3RICtion",
                        None,
                        Some(GateRows::JAVA_UUID_2),
                        2.0,
                    ));
                    Some("TTV_3RICtion")
                }
                "G14" => {
                    roster.push(GameServerReport::player_at(
                        "Notch",
                        None,
                        Some(GateRows::FLOODGATE_UUID_2),
                        2.0,
                    ));
                    Some("Notch")
                }
                _ => None,
            };

            let label = format!("{} {wire_label}", row.id);
            failures.extend(RoutingGate::check(&report, &label, roster, row.gamertag, decoy).await);
        }
    }

    assert!(failures.is_empty(), "{failures:#?}");
}

#[tokio::test]
async fn every_alias_path_is_heard() {
    let mut failures = Vec::new();

    // A beta.27 orphan under the Floodgate name already exists.
    let report = GameServerReport::new().await;
    report
        .registrar
        .create(".TTV_3RICtion", &common::Game::Minecraft, Some(GateRows::FLOODGATE_UUID))
        .await
        .expect("orphan");
    let roster = vec![
        GameServerReport::player_at(
            ".TTV_3RICtion",
            Some("TTV 3RICtion"),
            Some(GateRows::FLOODGATE_UUID),
            0.0,
        ),
        RoutingGate::listener(),
    ];
    failures.extend(RoutingGate::check(&report, "D2", roster, "TTV 3RICtion", None).await);

    // After a Floodgate session, the linked player joins from Java Edition directly.
    let report = GameServerReport::new().await;
    report
        .ingest(vec![GameServerReport::player(
            "JavaName",
            Some("XboxTag"),
            Some(GateRows::JAVA_UUID),
        )])
        .await;
    let roster = vec![
        GameServerReport::player_at("JavaName", None, Some(GateRows::JAVA_UUID), 0.0),
        RoutingGate::listener(),
    ];
    failures.extend(RoutingGate::check(&report, "E3", roster, "XboxTag", None).await);

    // A real Java account joins under a name a Floodgate claim already aliased. Its first
    // tick registers its own record under that name; on the next it must still be heard
    // as itself, not as the Floodgate player.
    let report = GameServerReport::new().await;
    report
        .ingest(vec![GameServerReport::player(
            "TTV_3RICtion",
            Some("TTV 3RICtion"),
            Some(GateRows::FLOODGATE_UUID),
        )])
        .await;
    report
        .ingest(vec![GameServerReport::player(
            "TTV_3RICtion",
            None,
            Some(GateRows::JAVA_UUID_2),
        )])
        .await;
    let roster = vec![
        GameServerReport::player_at("TTV_3RICtion", None, Some(GateRows::JAVA_UUID_2), 0.0),
        RoutingGate::listener(),
    ];
    failures.extend(RoutingGate::check(&report, "K03", roster, "TTV_3RICtion", None).await);

    assert!(failures.is_empty(), "{failures:#?}");
}

// A Java account named Steve stands far away; the Floodgate player whose gamertag is
// Steve stands beside the listener. If the Java entry owned the cache, the speaker would be
// out of range and the listener would hear nothing.
#[tokio::test]
async fn a_roster_with_two_players_named_alike_routes_the_xbox_verified_one() {
    let report = GameServerReport::new().await;
    let roster = vec![
        GameServerReport::player_at("Steve", None, Some(GateRows::JAVA_UUID_2), 10_000.0),
        GameServerReport::player_at("Steve", None, Some(GateRows::FLOODGATE_UUID), 0.0),
        RoutingGate::listener(),
    ];

    let failures = RoutingGate::check(&report, "K01", roster, "Steve", None).await;

    assert!(failures.is_empty(), "{failures:#?}");
}

// The gate's teeth: the beta.27 server's view of a space-mangled name, with no
// canonicalization, must leave
// the player unresolvable. If this ever passes silently, the gate proves nothing.
#[tokio::test]
async fn the_gate_detects_a_mismatched_name() {
    let roster = vec![
        GameServerReport::player_at(
            ".TTV_3RICtion",
            Some("TTV 3RICtion"),
            Some(GateRows::FLOODGATE_UUID),
            0.0,
        ),
        RoutingGate::listener(),
    ];
    let cache = RoutingFixture::player_cache(&roster).await;

    assert!(cache.get("minecraft:TTV 3RICtion").await.is_none());
    let delivered = RoutingGate::route(
        &cache,
        &["minecraft:TTV 3RICtion", "minecraft:Listener"],
        "minecraft:Listener",
    )
    .await
    .expect("listener resolves");
    assert!(delivered[0].is_none());
}
