use std::net::SocketAddr;

use bvc_relay_service::config::PlayCaConfig;
use bvc_relay_service::play::StepCaSigner;

use crate::harness::CsrFixture;

// Manual check against the development step-ca from `mise run play-ca`
// (docs/runbooks/play-ca-local.md). It needs a running CA, so it is ignored:
//
//   mise run test-relay -- --ignored step_ca_signs
//
// BVC_TEST_STEP_CA_URL, BVC_TEST_STEP_CA_CONNECT, BVC_TEST_STEP_CA_PROVISIONER,
// BVC_TEST_STEP_CA_KID, BVC_TEST_STEP_CA_KEY_PATH (PKCS#8 PEM) and
// BVC_TEST_STEP_CA_ROOT name the values `play-ca-init` prints.
#[tokio::test]
#[ignore = "needs a running step-ca"]
async fn step_ca_signs_a_registry_token() {
    let var = |name: &str| std::env::var(name).unwrap_or_else(|_| panic!("{name} is not set"));

    let config = PlayCaConfig {
        url: var("BVC_TEST_STEP_CA_URL"),
        connect: var("BVC_TEST_STEP_CA_CONNECT")
            .parse::<SocketAddr>()
            .expect("an address"),
        provisioner: var("BVC_TEST_STEP_CA_PROVISIONER"),
        provisioner_kid: var("BVC_TEST_STEP_CA_KID"),
        provisioner_key: std::fs::read_to_string(var("BVC_TEST_STEP_CA_KEY_PATH"))
            .expect("the key file reads"),
        root: var("BVC_TEST_STEP_CA_ROOT"),
    };
    let fqdn = "brave-otter-mesa.play.bedrockvc.stream";
    let csr = CsrFixture::pem(&CsrFixture::for_names(&[fqdn]));

    let chain = StepCaSigner::new(&config)
        .expect("the client builds")
        .sign(&csr, fqdn)
        .await
        .expect("step-ca signs");

    assert_eq!(chain.matches("BEGIN CERTIFICATE").count(), 2);
}
