use bvc_server_lib::config::{AcmeProviderKind, ApplicationConfig};
use bvc_server_lib::runtime::ca_cert::SanKeySet;
use bvc_server_lib::runtime::enrollment::EnrollmentStep;

// The assigned name has to reach `tls.names` before the local CA is signed and before
// the certificate order is built, because both read that list.
#[test]
fn applying_an_assignment_adds_the_name_to_tls_names() {
    let mut config = ApplicationConfig::default();

    EnrollmentStep::apply(
        &mut config,
        "creeper-diorite-badlands.bedrockvc.stream".to_string(),
    );

    assert!(
        config
            .server
            .tls
            .names
            .contains(&"creeper-diorite-badlands.bedrockvc.stream".to_string())
    );
}

// The default entries stay: the local CA presents itself as the QUIC leaf and must
// still carry them for a loopback client.
#[test]
fn applying_an_assignment_keeps_the_default_names() {
    let mut config = ApplicationConfig::default();

    EnrollmentStep::apply(&mut config, "assigned.bedrockvc.stream".to_string());

    assert!(config.server.tls.names.contains(&"localhost".to_string()));
}

// The certificate order carries the assigned name alone. Left to the fallback it
// would carry `localhost` too, which is not an IP, survives the filter, and fails the
// whole order.
#[test]
fn applying_an_assignment_scopes_the_certificate_order_to_the_assigned_name() {
    let mut config = ApplicationConfig::default();

    EnrollmentStep::apply(&mut config, "assigned.bedrockvc.stream".to_string());

    let acme = config.server.tls.acme.expect("an acme block is installed");
    assert_eq!(acme.provider, Some(AcmeProviderKind::BvcRelay));
    assert_eq!(
        acme.domains,
        Some(vec!["assigned.bedrockvc.stream".to_string()])
    );
}

// Applying twice is idempotent. A server re-reads its stored name on every boot, and
// a duplicate SAN entry would read to `SanKeySet` as drift and re-sign the CA on every
// start.
#[test]
fn applying_the_same_assignment_twice_does_not_duplicate_the_name() {
    let mut config = ApplicationConfig::default();
    EnrollmentStep::apply(&mut config, "assigned.bedrockvc.stream".to_string());

    EnrollmentStep::apply(&mut config, "assigned.bedrockvc.stream".to_string());

    let count = config
        .server
        .tls
        .names
        .iter()
        .filter(|n| *n == "assigned.bedrockvc.stream")
        .count();
    assert_eq!(count, 1);
}

// The regression guard for the startup ordering. A CA signed before the name was
// known omits it, self-corrects only on the NEXT boot via SAN drift, and logs nothing
// in between — so for one whole run the QUIC listener presents a leaf missing its own
// name.
#[test]
fn the_san_set_built_from_an_applied_assignment_contains_the_name() {
    let mut config = ApplicationConfig::default();
    EnrollmentStep::apply(&mut config, "assigned.bedrockvc.stream".to_string());

    let mut sans = config.server.tls.names.clone();
    sans.append(&mut config.server.tls.ips.clone());
    let set = SanKeySet::from_strings(&sans).expect("the san set builds");

    assert!(
        set.sorted()
            .iter()
            .any(|entry| entry.contains("assigned.bedrockvc.stream")),
        "the assigned name must be in the CA's SAN set on the first boot, not the second"
    );
}
