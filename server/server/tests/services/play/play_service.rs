//! The play-zone name and certificate lifecycle, against a scripted stand-in for the registry.
//!
//! Tests that wait on backoff or renewal pause the tokio clock, so ten days pass at once.
//! They use a database that owns no timers (see `DatabaseFixture::create_for_paused_clock`)
//! and pause only after it is connected.

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use bvc_server_lib::services::acme::{AcmeStorage, CertificateExpiry};
use bvc_server_lib::services::play::{
    PlayAuthority, PlayNameStore, PlayPreflightRequest, PlayService, ScriptedPlayAuthority,
};
use common::structs::relay::enroll::{EnrollRefuseReason, PlayPreflightOutcome};
use rcgen::{CertificateParams, CertificateSigningRequestParams, KeyPair, SanType};
use rustls::pki_types::CertificateSigningRequestDer;
use tempfile::TempDir;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::harness::DatabaseFixture;

const NAME: &str = "brave-otter-mesa-0042.play.bedrockvc.stream";

struct Rig {
    db: DatabaseFixture,
    certs: TempDir,
}

impl Rig {
    async fn new() -> Self {
        Self {
            db: DatabaseFixture::create_for_paused_clock()
                .await
                .expect("database fixture"),
            certs: tempfile::tempdir().expect("certs directory"),
        }
    }

    fn certs_path(&self) -> &str {
        self.certs.path().to_str().expect("utf-8 path")
    }

    fn service(&self, authority: &ScriptedPlayAuthority) -> PlayService {
        PlayService::new(
            PlayAuthority::Scripted(authority.clone()),
            Arc::new(self.db.connection.clone()),
            self.certs_path(),
        )
    }

    // Binds an ephemeral loopback port for the preflight responder.
    fn request() -> PlayPreflightRequest {
        PlayPreflightRequest {
            address: None,
            port: 443,
            quic_port: 443,
            listen: "127.0.0.1:0".parse().unwrap(),
        }
    }

    // Lets virtual time pass until the authority has been asked `count` times. A fixed sleep
    // would race the database, whose real I/O lets the paused clock jump ahead.
    async fn wait_for_preflights(authority: &ScriptedPlayAuthority, count: u32) {
        for _ in 0..600 {
            if authority.preflights() >= count {
                return;
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
        panic!("only {} preflights happened", authority.preflights());
    }

    // Runs `boot` in the background so a test can cancel it after virtual time passes.
    fn boot_in_background(
        service: PlayService,
        cancel: &CancellationToken,
    ) -> tokio::task::JoinHandle<anyhow::Result<(String, bvc_server_lib::services::acme::AcmeCertPaths)>>
    {
        let cancel = cancel.clone();
        tokio::spawn(async move { service.boot(&Rig::request(), &cancel).await })
    }

    fn storage(&self) -> AcmeStorage {
        PlayService::storage_for(
            self.certs_path(),
            Arc::new(self.db.connection.clone()),
            NAME,
        )
    }

    // Puts a certificate for `NAME` with the given validity into the play store.
    async fn store_certificate(&self, lived: time::Duration, remaining: time::Duration) {
        let key = KeyPair::generate().unwrap();
        let mut params = CertificateParams::new(vec![NAME.to_string()]).unwrap();
        params.not_before = time::OffsetDateTime::now_utc() - lived;
        params.not_after = time::OffsetDateTime::now_utc() + remaining;
        let pem = params.self_signed(&key).unwrap().pem();

        let storage = self.storage();
        storage.store_account_credentials("{}").await.unwrap();
        storage
            .store_certificate(&pem, &key.serialize_pem())
            .await
            .unwrap();
    }
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|window| window == needle)
}

#[tokio::test]
async fn no_stored_certificate_issues_before_returning_paths() {
    let rig = Rig::new().await;
    let authority = ScriptedPlayAuthority::new(NAME, 0);

    let paths = rig
        .service(&authority)
        .ensure_certificate(NAME, &CancellationToken::new())
        .await
        .expect("a certificate is issued");

    assert!(Path::new(&paths.certificate).exists());
    assert!(Path::new(&paths.key).exists());
    assert_eq!(authority.issued(), 1);
}

// A restart with a certificate in hand must not ask the registry again: it is rate limited,
// and a server restarted in a loop would spend the allowance.
#[tokio::test]
async fn a_valid_stored_certificate_returns_without_issuing() {
    let rig = Rig::new().await;
    let first = ScriptedPlayAuthority::new(NAME, 0);
    rig.service(&first)
        .ensure_certificate(NAME, &CancellationToken::new())
        .await
        .expect("first boot issues");

    // The durable copy is the database. A container with no volume has lost the files.
    std::fs::remove_dir_all(Path::new(rig.certs_path()).join("play")).expect("remove the files");

    let second = ScriptedPlayAuthority::new(NAME, 0);
    let paths = rig
        .service(&second)
        .ensure_certificate(NAME, &CancellationToken::new())
        .await
        .expect("second boot reuses");

    assert_eq!(second.issued(), 0);
    assert!(Path::new(&paths.certificate).exists());
    assert!(Path::new(&paths.key).exists());
}

// A certificate that expired while the server was stopped is replaced before
// any listener starts, rather than served or crash-looped on.
#[tokio::test]
async fn an_expired_stored_certificate_reissues_before_returning() {
    let rig = Rig::new().await;

    rig.store_certificate(time::Duration::days(16), time::Duration::days(-1))
        .await;

    let authority = ScriptedPlayAuthority::new(NAME, 0);
    let paths = rig
        .service(&authority)
        .ensure_certificate(NAME, &CancellationToken::new())
        .await
        .expect("the expired certificate is replaced");

    assert_eq!(authority.issued(), 1);
    let served = std::fs::read_to_string(&paths.certificate).unwrap();
    assert!(
        !CertificateExpiry::is_expired(&served).unwrap(),
        "the file a listener would read must hold the new certificate"
    );
}

#[tokio::test]
async fn issuance_retries_until_the_authority_answers() {
    let rig = Rig::new().await;
    let authority = ScriptedPlayAuthority::new(NAME, 2);
    let service = rig.service(&authority);
    tokio::time::pause();

    let started = tokio::time::Instant::now();
    service
        .ensure_certificate(NAME, &CancellationToken::new())
        .await
        .expect("issuance succeeds on the third try");

    assert_eq!(authority.issued(), 1);
    // Boot backs off 5 s, then 10 s.
    let waited = started.elapsed();
    assert!(
        waited >= Duration::from_secs(15) && waited < Duration::from_secs(16),
        "waited {waited:?}"
    );
}

#[tokio::test]
async fn cancel_stops_the_wait_with_an_error() {
    let rig = Rig::new().await;
    let authority = ScriptedPlayAuthority::new(NAME, u32::MAX);
    let service = Arc::new(rig.service(&authority));
    let cancel = CancellationToken::new();
    tokio::time::pause();

    let waiting = {
        let service = service.clone();
        let cancel = cancel.clone();
        tokio::spawn(async move { service.ensure_certificate(NAME, &cancel).await })
    };
    tokio::time::sleep(Duration::from_secs(1)).await;
    cancel.cancel();

    let error = waiting
        .await
        .unwrap()
        .expect_err("a cancelled wait is an error");
    assert!(
        error.to_string().contains("cancelled"),
        "the error must say why: {error}"
    );
    assert_eq!(authority.issued(), 0);
}

// The registry signs a request. It must never be handed the key it signs for.
#[tokio::test]
async fn the_csr_never_carries_the_private_key() {
    let rig = Rig::new().await;
    let authority = ScriptedPlayAuthority::new(NAME, 0);

    let paths = rig
        .service(&authority)
        .ensure_certificate(NAME, &CancellationToken::new())
        .await
        .unwrap();

    let requests = authority.requests();
    assert_eq!(requests.len(), 1);

    let parsed = CertificateSigningRequestParams::from_der(&CertificateSigningRequestDer::from(
        requests[0].clone(),
    ))
    .expect("the request parses as a CSR");
    assert_eq!(parsed.params.subject_alt_names.len(), 1);
    assert!(matches!(
        &parsed.params.subject_alt_names[0],
        SanType::DnsName(dns) if dns.as_str() == NAME
    ));

    let key = KeyPair::from_pem(&std::fs::read_to_string(&paths.key).unwrap()).unwrap();
    assert!(
        !contains(&requests[0], &key.serialized_der()),
        "the stored private key's DER appears in the request"
    );
}

#[tokio::test]
async fn a_returned_leaf_for_another_name_is_rejected() {
    let rig = Rig::new().await;
    let authority = ScriptedPlayAuthority::new_misissuing(NAME, "other-name.play.bedrockvc.stream");
    let service = rig.service(&authority);

    let error = service
        .issue(NAME)
        .await
        .expect_err("a leaf for another name is refused");

    assert!(error.to_string().contains("other names"), "{error}");
    assert!(
        rig.storage().load_certificate_any().await.unwrap().is_none(),
        "nothing may be stored from a refused leaf"
    );
    assert!(!Path::new(rig.certs_path()).join("play").join("key.pem").exists());
}

// Reviewed defect: a certificate with seconds left used to start the listeners and renew a
// minute later. Past two thirds of its life, it is replaced at boot when the registry answers.
#[tokio::test]
async fn a_certificate_past_its_renewal_point_is_replaced_at_boot() {
    let rig = Rig::new().await;
    // 12 of 15 days used: past the 10 day renewal point, 3 days from expiry.
    rig.store_certificate(time::Duration::days(12), time::Duration::days(3))
        .await;
    let authority = ScriptedPlayAuthority::new(NAME, 0);

    let paths = rig
        .service(&authority)
        .ensure_certificate(NAME, &CancellationToken::new())
        .await
        .unwrap();

    assert_eq!(authority.issued(), 1);
    let served = std::fs::read_to_string(&paths.certificate).unwrap();
    assert!(
        CertificateExpiry::renew_at(&served).unwrap() > std::time::SystemTime::now(),
        "the file must hold the new certificate"
    );
}

// The registry being down at boot must not stop a server whose certificate still works.
#[tokio::test]
async fn a_certificate_past_its_renewal_point_is_served_when_the_registry_is_down() {
    let rig = Rig::new().await;
    rig.store_certificate(time::Duration::days(12), time::Duration::days(3))
        .await;
    let authority = ScriptedPlayAuthority::new(NAME, u32::MAX);

    let paths = rig
        .service(&authority)
        .ensure_certificate(NAME, &CancellationToken::new())
        .await
        .expect("the stored certificate is still valid");

    assert_eq!(authority.issued(), 0);
    assert!(Path::new(&paths.certificate).exists());
}

// Reviewed defect C1: a name the registry released (30 days without a check-in) used to be
// retried for ever. The server forgets it and claims again.
#[tokio::test]
async fn a_name_the_registry_released_is_claimed_again() {
    let rig = Rig::new().await;
    PlayNameStore::write(&rig.db.connection, NAME).await.unwrap();
    let authority = ScriptedPlayAuthority::new_forgetful(NAME, 1);
    let service = rig.service(&authority);
    tokio::time::pause();

    let (name, paths) = service
        .boot(&Rig::request(), &CancellationToken::new())
        .await
        .expect("boot recovers");

    assert_eq!(name, NAME);
    assert!(Path::new(&paths.certificate).exists());
    assert_eq!(authority.claimed(), 1, "the stored name was dropped and claimed again");
    assert_eq!(authority.issued(), 1);
    assert_eq!(
        PlayNameStore::read(&rig.db.connection).await.unwrap(),
        Some(NAME.to_string())
    );
}

// Reviewed defect I1: ACME reads `acme_credential` row 1 as an account. Play must leave it
// and ACME's directory alone, or ACME fails to start on a server that once ran play.
#[tokio::test]
async fn play_leaves_the_acme_account_row_and_files_alone() {
    let rig = Rig::new().await;
    let authority = ScriptedPlayAuthority::new(NAME, 0);
    rig.service(&authority)
        .ensure_certificate(NAME, &CancellationToken::new())
        .await
        .unwrap();

    let acme = AcmeStorage::new(
        rig.certs_path(),
        Arc::new(rig.db.connection.clone()),
        "https://acme.example/directory".to_string(),
        vec![NAME.to_string()],
    );

    assert!(
        acme.load_account_credentials().await.unwrap().is_none(),
        "ACME must find no account row, so it registers one"
    );
    assert!(!Path::new(rig.certs_path()).join("acme").exists());
}

#[tokio::test]
async fn renewal_fires_at_two_thirds_and_signals() {
    let rig = Rig::new().await;
    let authority = ScriptedPlayAuthority::new(NAME, 0);
    let service = Arc::new(rig.service(&authority));
    service
        .ensure_certificate(NAME, &CancellationToken::new())
        .await
        .expect("first issuance");
    assert_eq!(authority.issued(), 1);

    let cancel = CancellationToken::new();
    let (renewed_tx, mut renewed_rx) = mpsc::channel(1);
    tokio::time::pause();
    let started = tokio::time::Instant::now();
    let handle = service
        .clone()
        .spawn_renewal(NAME.to_string(), cancel.clone(), renewed_tx);

    renewed_rx.recv().await.expect("a renewal is signalled");

    // Two thirds of 15 days is 10, give or take the second a certificate's validity rounds to.
    let elapsed = started.elapsed();
    assert!(
        elapsed >= Duration::from_secs(10 * 86400 - 5)
            && elapsed <= Duration::from_secs(10 * 86400 + 5),
        "renewal came after {elapsed:?}"
    );
    assert_eq!(authority.issued(), 2);

    cancel.cancel();
    handle.await.unwrap();
    assert!(
        renewed_rx.try_recv().is_err(),
        "exactly one renewal is signalled"
    );
}

fn timed_out() -> PlayPreflightOutcome {
    PlayPreflightOutcome::TimedOut {
        address: "203.0.113.9".to_string(),
        port: 443,
    }
}

// Nothing the registry hands out may be requested until the registry has reached this server.
#[tokio::test]
async fn a_failed_preflight_claims_nothing() {
    let rig = Rig::new().await;
    let authority = ScriptedPlayAuthority::new(NAME, 0).with_preflight(vec![timed_out()]);
    let cancel = CancellationToken::new();
    tokio::time::pause();

    let boot = Rig::boot_in_background(rig.service(&authority), &cancel);
    Rig::wait_for_preflights(&authority, 2).await;
    cancel.cancel();

    boot.await.unwrap().expect_err("a cancelled boot is an error");
    assert!(authority.preflights() >= 2, "{}", authority.preflights());
    assert_eq!(authority.claimed(), 0);
    assert_eq!(authority.issued(), 0);
}

#[tokio::test]
async fn a_passed_preflight_claims_and_issues() {
    let rig = Rig::new().await;
    let authority = ScriptedPlayAuthority::new(NAME, 0);
    tokio::time::pause();

    let (name, paths) = rig
        .service(&authority)
        .boot(&Rig::request(), &CancellationToken::new())
        .await
        .expect("boot succeeds");

    assert_eq!(name, NAME);
    assert!(Path::new(&paths.certificate).exists());
    assert_eq!(authority.preflights(), 1);
    assert_eq!(authority.claimed(), 1);
    assert_eq!(authority.issued(), 1);
}

#[tokio::test]
async fn a_valid_stored_certificate_skips_preflight() {
    let rig = Rig::new().await;
    PlayNameStore::write(&rig.db.connection, NAME).await.unwrap();
    rig.store_certificate(time::Duration::days(1), time::Duration::days(14))
        .await;
    // Any preflight would fail, so a boot that returns proves none ran.
    let authority = ScriptedPlayAuthority::new(NAME, 0).with_preflight(vec![timed_out()]);

    rig.service(&authority)
        .boot(&Rig::request(), &CancellationToken::new())
        .await
        .expect("boot succeeds");

    assert_eq!(authority.preflights(), 0);
    assert_eq!(authority.claimed(), 0);
    assert_eq!(authority.issued(), 0);
}

// A name is no proof the network still works. An expired certificate on a
// network the registry can no longer reach must not be replaced.
#[tokio::test]
async fn expired_certificate_on_an_unreachable_network_issues_nothing() {
    let rig = Rig::new().await;
    PlayNameStore::write(&rig.db.connection, NAME).await.unwrap();
    rig.store_certificate(time::Duration::days(16), time::Duration::days(-1))
        .await;
    let authority = ScriptedPlayAuthority::new(NAME, 0).with_preflight(vec![timed_out()]);
    let cancel = CancellationToken::new();
    tokio::time::pause();

    let boot = Rig::boot_in_background(rig.service(&authority), &cancel);
    Rig::wait_for_preflights(&authority, 2).await;
    cancel.cancel();

    boot.await.unwrap().expect_err("a cancelled boot is an error");
    assert_eq!(authority.issued(), 0);
}

// The registry forgets passed preflights when it restarts. The claim is
// refused for it, and the server recovers without the operator.
#[tokio::test]
async fn preflight_required_on_claim_runs_preflight_again() {
    let rig = Rig::new().await;
    let authority = ScriptedPlayAuthority::new(NAME, 0)
        .refusing_claims(EnrollRefuseReason::PreflightRequired, 1);
    tokio::time::pause();

    let (name, _) = rig
        .service(&authority)
        .boot(&Rig::request(), &CancellationToken::new())
        .await
        .expect("boot recovers");

    assert_eq!(name, NAME);
    assert_eq!(authority.preflights(), 2);
    assert_eq!(authority.claimed(), 2);
    assert_eq!(authority.issued(), 1);
}

#[tokio::test]
async fn a_stored_name_is_reused_without_a_claim() {
    let rig = Rig::new().await;
    PlayNameStore::write(&rig.db.connection, NAME).await.unwrap();
    // Any claim would fail, so an answer proves none was made.
    let authority = ScriptedPlayAuthority::new(NAME, u32::MAX);

    let name = rig
        .service(&authority)
        .name(&CancellationToken::new())
        .await
        .expect("the stored name is returned");

    assert_eq!(name, NAME);
    assert_eq!(authority.claimed(), 0);
}

#[tokio::test]
async fn a_claimed_name_is_stored_and_not_claimed_again() {
    let rig = Rig::new().await;
    let authority = ScriptedPlayAuthority::new(NAME, 1);
    tokio::time::pause();

    let first = rig
        .service(&authority)
        .name(&CancellationToken::new())
        .await
        .expect("the claim succeeds after a retry");
    let second = rig
        .service(&authority)
        .name(&CancellationToken::new())
        .await
        .expect("the second boot reuses the name");

    assert_eq!(first, NAME);
    assert_eq!(second, NAME);
    // One refused claim and one that succeeded. The second boot made none.
    assert_eq!(authority.claimed(), 2);
}

// The registry named how long to wait. Asking sooner spends the allowance it is protecting.
#[tokio::test]
async fn a_rate_limited_preflight_waits_the_time_the_registry_names() {
    let rig = Rig::new().await;
    let reachable = PlayPreflightOutcome::Reachable {
        address: "203.0.113.9".to_string(),
        port: 443,
    };
    let authority = ScriptedPlayAuthority::new(NAME, 0).with_preflight(vec![
        PlayPreflightOutcome::RateLimited {
            retry_after_secs: 120,
        },
        reachable,
    ]);
    tokio::time::pause();
    let started = tokio::time::Instant::now();

    rig.service(&authority)
        .preflight(&Rig::request(), &CancellationToken::new())
        .await
        .expect("the second preflight passes");

    assert_eq!(authority.preflights(), 2);
    assert!(started.elapsed() >= Duration::from_secs(120));
}

// The registry counts its per-node window from admission. A retry inside it is refused, so
// the first retry must land outside it.
#[tokio::test]
async fn the_first_preflight_retry_waits_out_the_registrys_node_window() {
    let rig = Rig::new().await;
    let refused = PlayPreflightOutcome::Refused {
        address: "203.0.113.9".to_string(),
        port: 443,
    };
    let reachable = PlayPreflightOutcome::Reachable {
        address: "203.0.113.9".to_string(),
        port: 443,
    };
    let authority =
        ScriptedPlayAuthority::new(NAME, 0).with_preflight(vec![refused, reachable]);
    tokio::time::pause();
    let started = tokio::time::Instant::now();

    rig.service(&authority)
        .preflight(&Rig::request(), &CancellationToken::new())
        .await
        .expect("the second preflight passes");

    // The registry's node window is 15 s.
    assert!(started.elapsed() >= Duration::from_secs(15));
}

// A request refused for want of a free probe has still opened the node window, so the wait
// is never shorter than the window.
#[test]
fn a_short_rate_limit_wait_is_raised_to_the_node_window() {
    assert_eq!(PlayService::rate_limited_wait(5), Duration::from_secs(15));
    assert_eq!(PlayService::rate_limited_wait(0), Duration::from_secs(15));
    assert_eq!(PlayService::rate_limited_wait(120), Duration::from_secs(120));
}

// A registry that passes preflight and still refuses the claim must not be asked in a tight
// loop. The first repeat is at once, and each one after it waits.
#[tokio::test]
async fn a_repeated_preflight_required_backs_off() {
    let rig = Rig::new().await;
    let authority = ScriptedPlayAuthority::new(NAME, 0)
        .refusing_claims(EnrollRefuseReason::PreflightRequired, 3);
    tokio::time::pause();
    let started = tokio::time::Instant::now();

    rig.service(&authority)
        .boot(&Rig::request(), &CancellationToken::new())
        .await
        .expect("boot recovers once the registry accepts the claim");

    assert_eq!(authority.preflights(), 4);
    // The second and third repeats wait 5 s and then 10 s.
    assert!(started.elapsed() >= Duration::from_secs(15));
}

// The registry accepted the request and went quiet. Cancelling must not wait out the
// request's own timeout while the responder holds the port.
#[tokio::test]
async fn cancelling_during_the_preflight_request_returns_at_once() {
    let rig = Rig::new().await;
    let authority = ScriptedPlayAuthority::new(NAME, 0).with_hanging_preflight();
    let cancel = CancellationToken::new();
    tokio::time::pause();
    let started = tokio::time::Instant::now();

    let service = rig.service(&authority);
    let task = tokio::spawn({
        let cancel = cancel.clone();
        async move { service.preflight(&Rig::request(), &cancel).await }
    });
    Rig::wait_for_preflights(&authority, 1).await;
    cancel.cancel();

    task.await
        .unwrap()
        .expect_err("a cancelled preflight is an error");
    assert!(started.elapsed() < PlayService::PREFLIGHT_TIMEOUT);
}
