use std::net::{IpAddr, SocketAddr};

use bvc_relay_service::config::{
    PlayCaConfig, PlayConfig, PlayDnsConfig, PlaySecondaryConfig, PlayTsigConfig,
};
use hickory_proto::rr::TSigner;

// A play configuration for tests: loopback DNS on an ephemeral port, no secondaries,
// and a certificate authority block nothing in the test contacts.
pub struct PlayFixture;

impl PlayFixture {
    pub const ZONE: &'static str = "play.bedrockvc.stream";
    pub const TSIG_NAME: &'static str = "xfr-key";
    // 32 bytes, base64.
    pub const TSIG_SECRET: &'static str = "MDEyMzQ1Njc4OWFiY2RlZjAxMjM0NTY3ODlhYmNkZWY=";
    // A different 32 bytes.
    pub const OTHER_SECRET: &'static str = "ZmVkY2JhOTg3NjU0MzIxMGZlZGNiYTk4NzY1NDMyMTA=";

    pub fn config() -> PlayConfig {
        PlayConfig {
            zone: Self::ZONE.to_string(),
            claims_per_address_per_day: 3,
            claims_per_hour: 200,
            release_after_days: 30,
            issuances_per_node_per_day: 5,
            // Tests check in repeatedly, from documentation addresses.
            min_check_in_interval_secs: 0,
            allow_private_probe_targets: true,
            dns: PlayDnsConfig {
                bind: vec!["127.0.0.1:0".parse().expect("a socket address")],
                nameservers: vec![
                    "ns1.bedrockvc.stream.".to_string(),
                    "ns2.bedrockvc.stream.".to_string(),
                ],
                primary: "ns1.bedrockvc.stream.".to_string(),
                hostmaster: "hostmaster.bedrockvc.stream.".to_string(),
                ttl: 60,
                secondaries: Vec::new(),
                tsig: None,
                notify_delay_secs: 5,
            },
            ca: PlayCaConfig {
                url: "https://ca.play.bedrockvc.stream:9000".to_string(),
                connect: "127.0.0.1:9000".parse().expect("a socket address"),
                provisioner: "registry".to_string(),
                provisioner_kid: "kid".to_string(),
                provisioner_key: String::new(),
                root: String::new(),
            },
        }
    }

    // One secondary that may pull from `transfer_from` and is notified at `notify`, with
    // the shared TSIG key.
    pub fn with_secondary(transfer_from: Vec<IpAddr>, notify: SocketAddr) -> PlayConfig {
        let mut config = Self::config();
        config.dns.secondaries = vec![PlaySecondaryConfig {
            transfer_from,
            notify,
        }];
        config.dns.tsig = Some(PlayTsigConfig {
            name: Self::TSIG_NAME.to_string(),
            secret: Self::TSIG_SECRET.to_string(),
        });
        config
    }

    pub fn signer() -> TSigner {
        Self::signer_with(Self::TSIG_SECRET)
    }

    pub fn wrong_signer() -> TSigner {
        Self::signer_with(Self::OTHER_SECRET)
    }

    fn signer_with(secret: &str) -> TSigner {
        PlayTsigConfig {
            name: Self::TSIG_NAME.to_string(),
            secret: secret.to_string(),
        }
        .signer()
        .expect("a valid key")
    }
}
