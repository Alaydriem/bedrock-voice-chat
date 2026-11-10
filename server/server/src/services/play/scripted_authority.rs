use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

use anyhow::{Result, anyhow};
use rcgen::{
    BasicConstraints, CertificateParams, CertificateSigningRequestParams, CertifiedIssuer,
    DistinguishedName, DnType, ExtendedKeyUsagePurpose, IsCa, KeyPair, KeyUsagePurpose, SanType,
};
use common::structs::relay::enroll::{EnrollRefuseReason, PlayPreflightOutcome};
use rustls::pki_types::CertificateSigningRequestDer;

use crate::services::EnrollmentError;

/// A stand-in for the registry that signs with an in-process CA.
///
/// It exists so a test can hand `PlayService` an authority it controls: fail the first N
/// calls, record what was asked, or answer for a different name than the one requested.
/// Clones share their state, so a test keeps a handle after giving one away.
#[derive(Clone)]
pub struct ScriptedPlayAuthority {
    name: String,
    // The name the signed leaf carries. Equal to `name` unless the test wants the authority
    // to answer for something else.
    signed_name: String,
    failures_left: Arc<AtomicU32>,
    // Certificate requests to refuse as "this node holds no play name", the registry's
    // answer once it has released the name.
    forgets_left: Arc<AtomicU32>,
    claims: Arc<AtomicU32>,
    issued: Arc<AtomicU32>,
    requests: Arc<Mutex<Vec<Vec<u8>>>>,
    issuer: Arc<CertifiedIssuer<'static, KeyPair>>,
    // Answers to preflight in order. The last one repeats.
    preflight_outcomes: Arc<Mutex<Vec<PlayPreflightOutcome>>>,
    preflights: Arc<AtomicU32>,
    // When set, a preflight never answers, like a registry that accepted the request and went
    // quiet.
    preflight_hangs: Arc<AtomicBool>,
    // Claims to refuse with `claim_refusal` before answering.
    claim_refusal: Arc<Mutex<EnrollRefuseReason>>,
    claim_refusals_left: Arc<AtomicU32>,
}

impl ScriptedPlayAuthority {
    // Matches the registry's certificate lifetime.
    const LEAF_LIFETIME: time::Duration = time::Duration::hours(360);

    /// Fails the first `failures_before_success` calls to `claim` and `issue`, then answers.
    pub fn new(name: &str, failures_before_success: u32) -> Self {
        Self::answering_for(name, name, failures_before_success)
    }

    /// Claims `name` and signs every request for `signed_name` instead.
    pub fn new_misissuing(name: &str, signed_name: &str) -> Self {
        Self::answering_for(name, signed_name, 0)
    }

    /// Refuses the first `forgets` certificate requests as `NoPlayName`, then signs.
    pub fn new_forgetful(name: &str, forgets: u32) -> Self {
        let authority = Self::answering_for(name, name, 0);
        authority.forgets_left.store(forgets, Ordering::SeqCst);
        authority
    }

    fn answering_for(name: &str, signed_name: &str, failures: u32) -> Self {
        let mut params = CertificateParams::default();
        let mut subject = DistinguishedName::new();
        subject.push(DnType::CommonName, "BVC scripted play authority");
        params.distinguished_name = subject;
        params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        params.key_usages = vec![KeyUsagePurpose::KeyCertSign, KeyUsagePurpose::CrlSign];

        let key = KeyPair::generate().expect("a signing key generates");
        let issuer = CertifiedIssuer::self_signed(params, key).expect("the CA certificate signs");

        Self {
            name: name.to_string(),
            signed_name: signed_name.to_string(),
            failures_left: Arc::new(AtomicU32::new(failures)),
            forgets_left: Arc::new(AtomicU32::new(0)),
            claims: Arc::new(AtomicU32::new(0)),
            issued: Arc::new(AtomicU32::new(0)),
            requests: Arc::new(Mutex::new(Vec::new())),
            issuer: Arc::new(issuer),
            preflight_outcomes: Arc::new(Mutex::new(Vec::new())),
            preflights: Arc::new(AtomicU32::new(0)),
            preflight_hangs: Arc::new(AtomicBool::new(false)),
            claim_refusal: Arc::new(Mutex::new(EnrollRefuseReason::Internal)),
            claim_refusals_left: Arc::new(AtomicU32::new(0)),
        }
    }

    /// Answers each preflight with the next of `outcomes`, repeating the last. With none
    /// scripted, every preflight is reachable.
    pub fn with_preflight(self, outcomes: Vec<PlayPreflightOutcome>) -> Self {
        if let Ok(mut scripted) = self.preflight_outcomes.lock() {
            *scripted = outcomes;
        }
        self
    }

    /// Never answers a preflight.
    pub fn with_hanging_preflight(self) -> Self {
        self.preflight_hangs.store(true, Ordering::SeqCst);
        self
    }

    /// Refuses the next `count` claims with `reason`.
    pub fn refusing_claims(self, reason: EnrollRefuseReason, count: u32) -> Self {
        if let Ok(mut scripted) = self.claim_refusal.lock() {
            *scripted = reason;
        }
        self.claim_refusals_left.store(count, Ordering::SeqCst);
        self
    }

    /// How many preflights have been asked for.
    pub fn preflights(&self) -> u32 {
        self.preflights.load(Ordering::SeqCst)
    }

    pub async fn preflight(&self) -> Result<PlayPreflightOutcome> {
        let index = self.preflights.fetch_add(1, Ordering::SeqCst) as usize;
        if self.preflight_hangs.load(Ordering::SeqCst) {
            std::future::pending::<()>().await;
        }
        let outcomes = self
            .preflight_outcomes
            .lock()
            .map_err(|_| anyhow!("the scripted preflight outcomes are poisoned"))?;

        Ok(outcomes
            .get(index)
            .or_else(|| outcomes.last())
            .cloned()
            .unwrap_or(PlayPreflightOutcome::Reachable {
                address: "127.0.0.1".to_string(),
                port: 0,
            }))
    }

    /// How many certificates have been signed.
    pub fn issued(&self) -> u32 {
        self.issued.load(Ordering::SeqCst)
    }

    /// How many times a name has been claimed, failed attempts included.
    pub fn claimed(&self) -> u32 {
        self.claims.load(Ordering::SeqCst)
    }

    /// Every certificate request received, as the bytes that arrived.
    pub fn requests(&self) -> Vec<Vec<u8>> {
        self.requests
            .lock()
            .map(|requests| requests.clone())
            .unwrap_or_default()
    }

    pub async fn claim(&self) -> Result<String> {
        self.claims.fetch_add(1, Ordering::SeqCst);
        let refusing = self
            .claim_refusals_left
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |left| left.checked_sub(1))
            .is_ok();
        if refusing {
            let reason = *self
                .claim_refusal
                .lock()
                .map_err(|_| anyhow!("the scripted claim refusal is poisoned"))?;
            return Err(EnrollmentError::refused(reason).into());
        }
        self.fail_while_scripted()?;

        Ok(self.name.clone())
    }

    pub async fn issue(&self, csr_der: Vec<u8>) -> Result<String> {
        if let Ok(mut requests) = self.requests.lock() {
            requests.push(csr_der.clone());
        }
        let forgetting = self
            .forgets_left
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |left| left.checked_sub(1))
            .is_ok();
        if forgetting {
            return Err(
                EnrollmentError::refused(EnrollRefuseReason::NoPlayName).into()
            );
        }
        self.fail_while_scripted()?;

        let mut request =
            CertificateSigningRequestParams::from_der(&CertificateSigningRequestDer::from(csr_der))
                .map_err(|e| anyhow!("the request does not parse: {e}"))?;

        let now = time::OffsetDateTime::now_utc();
        request.params.not_before = now;
        request.params.not_after = now + Self::LEAF_LIFETIME;
        request.params.is_ca = IsCa::NoCa;
        request.params.extended_key_usages = vec![ExtendedKeyUsagePurpose::ServerAuth];
        request.params.subject_alt_names = vec![SanType::DnsName(
            self.signed_name
                .as_str()
                .try_into()
                .map_err(|e| anyhow!("the name is not a DNS name: {e}"))?,
        )];

        let leaf = request
            .signed_by(&self.issuer)
            .map_err(|e| anyhow!("signing the leaf: {e}"))?;
        self.issued.fetch_add(1, Ordering::SeqCst);

        Ok(format!("{}{}", leaf.pem(), self.issuer.pem()))
    }

    fn fail_while_scripted(&self) -> Result<()> {
        let failing = self
            .failures_left
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |left| left.checked_sub(1))
            .is_ok();

        if failing {
            return Err(anyhow!("the scripted authority is set to fail"));
        }

        Ok(())
    }
}
