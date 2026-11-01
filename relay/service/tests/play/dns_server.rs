use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::sync::Arc;

use bvc_relay_service::play::{PlayDnsServer, PlayError, PlayZone};
use hickory_proto::op::{Message, OpCode, ResponseCode};
use hickory_proto::rr::{RData, RecordType};
use tokio_util::sync::CancellationToken;

use crate::harness::{DnsProbe, PlayFixture};

const LABEL: &str = "brave-otter-mesa";

async fn running(zone: Arc<PlayZone>) -> (SocketAddr, CancellationToken) {
    let server = PlayDnsServer::bind(zone, &["127.0.0.1:0".parse().expect("an address")])
        .await
        .expect("binds");
    let address = server.local_addrs()[0];
    let cancel = CancellationToken::new();
    server.spawn(cancel.clone());
    (address, cancel)
}

fn zone() -> Arc<PlayZone> {
    PlayZone::new_shared(PlayFixture::ZONE.to_string(), PlayFixture::config().dns)
}

fn name(label: &str) -> String {
    format!("{label}.{}.", PlayFixture::ZONE)
}

fn v4() -> IpAddr {
    IpAddr::V4(Ipv4Addr::new(198, 51, 100, 4))
}

#[tokio::test]
async fn a_published_v4_name_answers_a_with_ttl_60_and_aa() {
    let zone = zone();
    zone.publish(LABEL, v4(), 1);
    let (server, _cancel) = running(zone).await;

    let reply = DnsProbe::ask(server, &name(LABEL), RecordType::A).await;

    assert_eq!(reply.metadata.response_code, ResponseCode::NoError);
    assert!(reply.metadata.authoritative);
    assert_eq!(reply.answers.len(), 1);
    assert_eq!(reply.answers[0].ttl, 60);
    assert!(matches!(&reply.answers[0].data, RData::A(a) if a.0 == Ipv4Addr::new(198, 51, 100, 4)));
}

#[tokio::test]
async fn a_published_v6_name_answers_aaaa() {
    let zone = zone();
    let address = Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 7);
    zone.publish(LABEL, IpAddr::V6(address), 1);
    let (server, _cancel) = running(zone).await;

    let reply = DnsProbe::ask(server, &name(LABEL), RecordType::AAAA).await;

    assert_eq!(reply.answers.len(), 1);
    assert!(matches!(&reply.answers[0].data, RData::AAAA(a) if a.0 == address));
}

#[tokio::test]
async fn an_aaaa_query_for_a_v4_name_is_nodata_with_soa() {
    let zone = zone();
    zone.publish(LABEL, v4(), 1);
    let (server, _cancel) = running(zone).await;

    let reply = DnsProbe::ask(server, &name(LABEL), RecordType::AAAA).await;

    assert_eq!(reply.metadata.response_code, ResponseCode::NoError);
    assert!(reply.metadata.authoritative);
    assert!(reply.answers.is_empty());
    assert!(matches!(reply.authorities[0].data, RData::SOA(_)));
}

#[tokio::test]
async fn an_any_query_returns_the_address_record() {
    let zone = zone();
    zone.publish(LABEL, v4(), 1);
    let (server, _cancel) = running(zone).await;

    let reply = DnsProbe::ask(server, &name(LABEL), RecordType::ANY).await;

    assert_eq!(reply.answers.len(), 1);
    assert!(matches!(reply.answers[0].data, RData::A(_)));
}

#[tokio::test]
async fn an_unknown_name_is_nxdomain_with_soa() {
    let (server, _cancel) = running(zone()).await;

    let reply = DnsProbe::ask(server, &name("no-such-name"), RecordType::A).await;

    assert_eq!(reply.metadata.response_code, ResponseCode::NXDomain);
    assert!(reply.metadata.authoritative);
    assert!(matches!(reply.authorities[0].data, RData::SOA(_)));
}

#[tokio::test]
async fn a_withdrawn_name_is_nxdomain() {
    let zone = zone();
    zone.publish(LABEL, v4(), 1);
    zone.withdraw(LABEL, 2);
    let (server, _cancel) = running(zone).await;

    let reply = DnsProbe::ask(server, &name(LABEL), RecordType::A).await;

    assert_eq!(reply.metadata.response_code, ResponseCode::NXDomain);
}

#[tokio::test]
async fn a_name_outside_the_zone_is_refused() {
    let (server, _cancel) = running(zone()).await;

    let reply = DnsProbe::ask(server, "www.example.com.", RecordType::A).await;

    assert_eq!(reply.metadata.response_code, ResponseCode::Refused);
    assert!(reply.metadata.authoritative);
}

#[tokio::test]
async fn an_opcode_other_than_query_is_not_implemented() {
    let (server, _cancel) = running(zone()).await;
    let mut request = DnsProbe::query(&name(LABEL), RecordType::A);
    request.metadata.op_code = OpCode::Update;

    let reply = Message::from_vec(&DnsProbe::udp(server, &request.to_vec().expect("encodes")).await)
        .expect("decodes");

    assert_eq!(reply.metadata.response_code, ResponseCode::NotImp);
    assert!(reply.metadata.authoritative);
}

// Resolvers randomise the case of the name they ask for (0x20) and compare the echo, so
// the question has to come back byte for byte as it was sent.
#[tokio::test]
async fn mixed_case_and_trailing_dot_answer_and_echo_the_question() {
    let zone = zone();
    zone.publish(LABEL, v4(), 1);
    let (server, _cancel) = running(zone).await;
    let request = DnsProbe::query("Brave-Otter-Mesa.PLAY.bedrockvc.stream.", RecordType::A)
        .to_vec()
        .expect("encodes");

    let raw = DnsProbe::udp(server, &request).await;
    let reply = Message::from_vec(&raw).expect("decodes");

    assert_eq!(reply.answers.len(), 1);
    assert!(matches!(reply.answers[0].data, RData::A(_)));
    let question = request.len() - 12;
    assert_eq!(&raw[12..12 + question], &request[12..]);
}

#[tokio::test]
async fn the_zone_apex_answers_soa_and_ns() {
    let zone = zone();
    zone.load(Vec::new(), 42);
    let (server, _cancel) = running(zone).await;

    let soa = DnsProbe::ask(server, "play.bedrockvc.stream.", RecordType::SOA).await;
    let ns = DnsProbe::ask(server, "play.bedrockvc.stream.", RecordType::NS).await;

    assert!(matches!(&soa.answers[0].data, RData::SOA(soa) if soa.serial == 42));
    assert_eq!(ns.answers.len(), 2);
    assert!(ns.answers.iter().all(|r| matches!(r.data, RData::NS(_))));
}

#[tokio::test]
async fn tcp_answers_the_same_as_udp() {
    let zone = zone();
    zone.publish(LABEL, v4(), 1);
    let (server, _cancel) = running(zone).await;
    let request = DnsProbe::query(&name(LABEL), RecordType::A)
        .to_vec()
        .expect("encodes");

    let over_tcp = Message::from_vec(&DnsProbe::tcp(server, &request).await).expect("decodes");
    let over_udp = Message::from_vec(&DnsProbe::udp(server, &request).await).expect("decodes");

    assert_eq!(over_tcp.answers, over_udp.answers);
    assert_eq!(over_tcp.metadata.response_code, over_udp.metadata.response_code);
    assert!(over_tcp.metadata.authoritative);
}

// systemd-resolved holds 127.0.0.53:53. A registry that carried on without DNS would
// look healthy while no play name resolved.
#[tokio::test]
async fn a_taken_address_fails_bind_and_names_it() {
    let taken = tokio::net::UdpSocket::bind("127.0.0.1:0").await.expect("binds");
    let address = taken.local_addr().expect("an address");

    let error = match PlayDnsServer::bind(zone(), &[address]).await {
        Ok(_) => panic!("binding a taken address must fail"),
        Err(e) => e,
    };

    assert!(matches!(error, PlayError::Bind { .. }));
    assert!(error.to_string().contains(&address.to_string()));
}

#[tokio::test]
async fn a_lower_serial_is_ignored() {
    let zone = zone();
    zone.publish(LABEL, v4(), 10);
    zone.withdraw(LABEL, 9);

    assert_eq!(zone.lookup(LABEL), Some(v4()));
    assert_eq!(zone.serial(), 10);
}

#[tokio::test]
async fn the_zone_announces_a_serial_change_once_it_grows() {
    let zone = zone();
    let mut changes = zone.subscribe();

    zone.publish(LABEL, v4(), 5);
    changes.changed().await.expect("a change");
    assert_eq!(*changes.borrow_and_update(), 5);

    zone.publish("other-name-here", v4(), 5);
    assert!(!changes.has_changed().expect("open"));
}
