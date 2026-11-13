use bvc_relay_service::play::{PlayZone, ZoneLocation};

use crate::harness::PlayFixture;

#[test]
fn locate_classifies_names_under_the_zone() {
    let zone = PlayZone::new(PlayFixture::ZONE.to_string(), PlayFixture::config().dns);

    assert_eq!(
        zone.locate("brave-otter.play.bedrockvc.stream."),
        ZoneLocation::Host("brave-otter".to_string())
    );
    assert_eq!(
        zone.locate("_bvc._tcp.brave-otter.play.bedrockvc.stream"),
        ZoneLocation::Service("brave-otter".to_string())
    );
    assert_eq!(
        zone.locate("_TCP.Brave-Otter.play.bedrockvc.stream"),
        ZoneLocation::EmptyNonTerminal("brave-otter".to_string())
    );
    assert_eq!(
        zone.locate("_bvc._udp.brave-otter.play.bedrockvc.stream"),
        ZoneLocation::Host("_bvc._udp.brave-otter".to_string())
    );
    assert_eq!(
        zone.locate("_bvc._tcp.a.b.play.bedrockvc.stream"),
        ZoneLocation::Host("_bvc._tcp.a.b".to_string())
    );
    assert_eq!(zone.locate("play.bedrockvc.stream"), ZoneLocation::Apex);
    assert_eq!(zone.locate("example.com"), ZoneLocation::Outside);
}
