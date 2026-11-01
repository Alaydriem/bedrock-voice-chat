use std::net::{IpAddr, Ipv4Addr};
use std::sync::Arc;
use std::time::Duration;

use bvc_relay_service::config::PlayConfig;
use bvc_relay_service::play::{PlayZone, PlayZoneNotifier};
use hickory_proto::op::{Message, MessageType};
use hickory_proto::rr::RData;
use tokio::net::UdpSocket;
use tokio_util::sync::CancellationToken;

use crate::harness::{DnsProbe, PlayFixture};

fn address(last: u8) -> IpAddr {
    IpAddr::V4(Ipv4Addr::new(198, 51, 100, last))
}

// A bound socket standing in for the secondary, and the notifier pointed at it.
async fn secondary(delay_secs: u64) -> (UdpSocket, PlayConfig) {
    let socket = UdpSocket::bind("127.0.0.1:0").await.expect("binds");
    let mut config = PlayFixture::with_secondary(
        vec![IpAddr::V4(Ipv4Addr::LOCALHOST)],
        socket.local_addr().expect("an address"),
    );
    config.dns.notify_delay_secs = delay_secs;
    (socket, config)
}

fn start(config: &PlayConfig) -> (Arc<PlayZone>, CancellationToken) {
    let zone = PlayZone::new_shared(config.zone.clone(), config.dns.clone());
    let cancel = CancellationToken::new();
    PlayZoneNotifier::new(zone.clone(), config.dns.clone()).spawn(cancel.clone());
    (zone, cancel)
}

async fn next_datagram(socket: &UdpSocket, within: Duration) -> Option<(Vec<u8>, std::net::SocketAddr)> {
    let mut buffer = vec![0u8; 2048];
    let (length, from) = tokio::time::timeout(within, socket.recv_from(&mut buffer))
        .await
        .ok()?
        .ok()?;
    buffer.truncate(length);
    Some((buffer, from))
}

#[tokio::test]
async fn three_changes_inside_the_delay_send_one_notify_with_the_last_serial() {
    let (socket, config) = secondary(1).await;
    let (zone, cancel) = start(&config);

    zone.publish("name-a", address(1), 10);
    zone.publish("name-b", address(2), 11);
    zone.publish("name-c", address(3), 12);

    let (bytes, from) = next_datagram(&socket, Duration::from_secs(5))
        .await
        .expect("a NOTIFY arrives");
    let notify = Message::from_vec(&bytes).expect("decodes");
    assert!(DnsProbe::is_notify(&notify));
    assert!(matches!(&notify.answers[0].data, RData::SOA(soa) if soa.serial == 12));
    PlayFixture::signer()
        .verify_message_byte(&bytes, None, true)
        .expect("the NOTIFY is signed with the shared key");

    // Acknowledge, so the notifier stops retrying, then expect silence.
    let mut ack = notify.clone();
    ack.metadata.message_type = MessageType::Response;
    socket
        .send_to(&ack.to_vec().expect("encodes"), from)
        .await
        .expect("acknowledges");
    assert!(next_datagram(&socket, Duration::from_millis(2500)).await.is_none());

    cancel.cancel();
}

#[tokio::test]
async fn an_unacknowledged_notify_is_retried() {
    let (socket, config) = secondary(0).await;
    let (zone, cancel) = start(&config);

    zone.publish("name-a", address(1), 10);

    let first = next_datagram(&socket, Duration::from_secs(5)).await.expect("a NOTIFY");
    let second = next_datagram(&socket, Duration::from_secs(5)).await.expect("a retry");
    let first = Message::from_vec(&first.0).expect("decodes");
    let second = Message::from_vec(&second.0).expect("decodes");

    assert!(DnsProbe::is_notify(&second));
    assert_eq!(first.metadata.id, second.metadata.id);

    cancel.cancel();
}

// The first secondary never answers. Told one after another, the second would wait out
// the first's whole retry schedule.
#[tokio::test]
async fn a_silent_secondary_does_not_delay_the_others() {
    let (silent, mut config) = secondary(0).await;
    let responsive = UdpSocket::bind("127.0.0.1:0").await.expect("binds");
    config.dns.secondaries.push(bvc_relay_service::config::PlaySecondaryConfig {
        transfer_from: vec![IpAddr::V4(Ipv4Addr::LOCALHOST)],
        notify: responsive.local_addr().expect("an address"),
    });
    let (zone, cancel) = start(&config);

    zone.publish("name-a", address(1), 10);

    assert!(next_datagram(&silent, Duration::from_secs(3)).await.is_some());
    assert!(next_datagram(&responsive, Duration::from_secs(3)).await.is_some());

    cancel.cancel();
}

#[tokio::test]
async fn no_secondaries_means_no_notifier() {
    let config = PlayFixture::config();
    let zone = PlayZone::new_shared(config.zone.clone(), config.dns.clone());

    let handle = PlayZoneNotifier::new(zone, config.dns.clone()).spawn(CancellationToken::new());

    tokio::time::timeout(Duration::from_secs(2), handle)
        .await
        .expect("the notifier task ends at once")
        .expect("without a panic");
}
