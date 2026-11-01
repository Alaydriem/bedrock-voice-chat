use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use bvc_relay_service::config::PlayConfig;
use bvc_relay_service::play::{PlayDnsServer, PlayZone};
use hickory_proto::op::{Message, ResponseCode};
use hickory_proto::rr::{RData, RecordType};
use tokio_util::sync::CancellationToken;

use crate::harness::{DnsProbe, PlayFixture};

fn loopback() -> IpAddr {
    IpAddr::V4(Ipv4Addr::LOCALHOST)
}

fn notify_target() -> SocketAddr {
    "127.0.0.1:9".parse().expect("an address")
}

async fn serve(config: PlayConfig, names: usize) -> (SocketAddr, CancellationToken) {
    let zone = PlayZone::new_shared(config.zone.clone(), config.dns.clone());
    let records = (0..names)
        .map(|i| {
            (
                format!("name-{i}"),
                IpAddr::V4(Ipv4Addr::new(10, (i / 65_536) as u8, (i / 256 % 256) as u8, (i % 256) as u8)),
            )
        })
        .collect();
    zone.load(records, 1000);

    let server = PlayDnsServer::bind(zone, &config.dns.bind)
        .await
        .expect("binds");
    let address = server.local_addrs()[0];
    let cancel = CancellationToken::new();
    server.spawn(cancel.clone());
    (address, cancel)
}

fn permitted() -> PlayConfig {
    PlayFixture::with_secondary(vec![loopback()], notify_target())
}

#[tokio::test]
async fn a_signed_axfr_from_a_secondary_returns_soa_records_soa() {
    let (server, _cancel) = serve(permitted(), 3).await;
    let signer = PlayFixture::signer();

    let messages = DnsProbe::transfer(server, RecordType::AXFR, Some(&signer), true).await;
    let records = DnsProbe::records(&messages);

    assert!(matches!(&records[0].data, RData::SOA(soa) if soa.serial == 1000));
    assert!(matches!(&records[records.len() - 1].data, RData::SOA(soa) if soa.serial == 1000));
    let addresses = records.iter().filter(|r| matches!(r.data, RData::A(_))).count();
    let name_servers = records.iter().filter(|r| matches!(r.data, RData::NS(_))).count();
    assert_eq!(addresses, 3);
    assert_eq!(name_servers, 2);
    assert_eq!(records.len(), 1 + 2 + 3 + 1);
}

#[tokio::test]
async fn an_axfr_of_5000_names_spans_several_messages_and_loses_none() {
    let (server, _cancel) = serve(permitted(), 5000).await;
    let signer = PlayFixture::signer();

    let messages = DnsProbe::transfer(server, RecordType::AXFR, Some(&signer), true).await;
    let records = DnsProbe::records(&messages);

    assert!(messages.len() > 1, "{} message", messages.len());
    for message in &messages {
        assert!(message.to_vec().expect("encodes").len() <= 16 * 1024);
    }
    let mut names: Vec<String> = records
        .iter()
        .filter(|r| matches!(r.data, RData::A(_)))
        .map(|r| r.name.to_string())
        .collect();
    names.sort();
    names.dedup();
    assert_eq!(names.len(), 5000);
    assert!(matches!(records[0].data, RData::SOA(_)));
    assert!(matches!(records[records.len() - 1].data, RData::SOA(_)));
}

#[tokio::test]
async fn an_unsigned_axfr_is_refused() {
    let (server, _cancel) = serve(permitted(), 3).await;

    let messages = DnsProbe::transfer(server, RecordType::AXFR, None, false).await;

    assert_eq!(messages[0].metadata.response_code, ResponseCode::Refused);
    assert!(messages[0].answers.is_empty());
}

#[tokio::test]
async fn an_axfr_signed_with_the_wrong_key_is_refused() {
    let (server, _cancel) = serve(permitted(), 3).await;
    let signer = PlayFixture::wrong_signer();

    let messages = DnsProbe::transfer(server, RecordType::AXFR, Some(&signer), false).await;

    assert_eq!(messages[0].metadata.response_code, ResponseCode::Refused);
    assert!(messages[0].answers.is_empty());
}

// The client is on 127.0.0.1 and the only listed secondary is elsewhere.
#[tokio::test]
async fn an_axfr_from_an_unlisted_address_is_refused() {
    let elsewhere = IpAddr::V4(Ipv4Addr::new(203, 0, 113, 99));
    let (server, _cancel) =
        serve(PlayFixture::with_secondary(vec![elsewhere], notify_target()), 3).await;
    let signer = PlayFixture::signer();

    let messages = DnsProbe::transfer(server, RecordType::AXFR, Some(&signer), false).await;

    assert_eq!(messages[0].metadata.response_code, ResponseCode::Refused);
    assert!(messages[0].answers.is_empty());
}

#[tokio::test]
async fn an_axfr_over_udp_is_refused() {
    let (server, _cancel) = serve(permitted(), 3).await;
    let signer = PlayFixture::signer();
    let mut request = DnsProbe::query("play.bedrockvc.stream.", RecordType::AXFR);
    request.finalize(&signer, DnsProbe::now()).expect("signs");

    let reply = Message::from_vec(
        &DnsProbe::udp(server, &request.to_vec().expect("encodes")).await,
    )
    .expect("decodes");

    assert_eq!(reply.metadata.response_code, ResponseCode::Refused);
    assert!(reply.answers.is_empty());
}

#[tokio::test]
async fn an_ixfr_returns_the_full_zone() {
    let (server, _cancel) = serve(permitted(), 3).await;
    let signer = PlayFixture::signer();

    let messages = DnsProbe::transfer(server, RecordType::IXFR, Some(&signer), true).await;
    let records = DnsProbe::records(&messages);

    assert!(matches!(records[0].data, RData::SOA(_)));
    assert!(matches!(records[records.len() - 1].data, RData::SOA(_)));
    assert_eq!(
        records.iter().filter(|r| matches!(r.data, RData::A(_))).count(),
        3
    );
}

// A secondary's SOA check is signed too, and a reply it cannot verify is dropped.
#[tokio::test]
async fn a_signed_soa_query_gets_a_signed_answer() {
    let (server, _cancel) = serve(permitted(), 3).await;
    let signer = PlayFixture::signer();
    let mut request = DnsProbe::query("play.bedrockvc.stream.", RecordType::SOA);
    let mut verifier = request
        .finalize(&signer, DnsProbe::now())
        .expect("signs")
        .expect("a verifier");

    let raw = DnsProbe::udp(server, &request.to_vec().expect("encodes")).await;
    verifier.verify(&raw).expect("the answer carries a valid signature");
    let reply = Message::from_vec(&raw).expect("decodes");

    assert!(matches!(&reply.answers[0].data, RData::SOA(soa) if soa.serial == 1000));
}
