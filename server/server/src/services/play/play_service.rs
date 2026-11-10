use std::sync::Arc;
use std::time::{Duration, SystemTime};

use anyhow::{Context, Result, anyhow, bail};
use common::curia;
use common::structs::relay::enroll::{EnrollRefuseReason, PlayPreflightOutcome};
use rcgen::{CertificateParams, DistinguishedName, KeyPair, PKCS_ECDSA_P256_SHA256};
use sea_orm::DatabaseConnection;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use super::{
    PlayAuthority, PlayBackoff, PlayNameStore, PlayPreflightReport, PlayPreflightRequest,
    PlayPreflightResponder, PreflightLogGate,
};
use crate::services::{CurrentNonce, EnrollmentError};
use crate::services::acme::{AcmeCertPaths, AcmeStorage, CertificateExpiry};

/// Takes a play-zone name and a certificate for it from the registry, and renews it.
///
/// The name is claimed once and kept in the database. The certificate is requested with a
/// key generated here: the registry receives a certificate request and never the key, and
/// the key never leaves this process except into the store beside the certificate.
pub struct PlayService {
    authority: PlayAuthority,
    conn: Arc<DatabaseConnection>,
    certs_path: String,
}

impl PlayService {
    /// The key the certificate store files this server's play material under, in the place a
    /// directory URL takes for ACME. A certificate stored for another source never matches it.
    pub const STORAGE_KEY: &'static str = "bvc-play";

    // Play keeps its files and its database row apart from ACME's. ACME reads its row as an
    // account, so anything else written there would stop it starting.
    const STORAGE_DIR: &'static str = "play";
    const STORAGE_ROW: i32 = 2;

    // Boot waits are short: the operator is watching the console and the listeners cannot
    // start without a certificate.
    const BOOT_BACKOFF_INITIAL: Duration = Duration::from_secs(5);
    const BOOT_BACKOFF_CAP: Duration = Duration::from_secs(300);

    // Renewal has five days of margin, so retries are patient.
    const RENEWAL_BACKOFF_INITIAL: Duration = Duration::from_secs(60);
    const RENEWAL_BACKOFF_CAP: Duration = Duration::from_secs(3600);

    // Never renew more often than this, whatever the stored certificate says. A certificate
    // whose lifetime the clock reads as already two thirds spent would otherwise be renewed in
    // a loop.
    const MIN_RENEWAL_GAP: Duration = Duration::from_secs(60);

    // A registry that accepts a connection and never answers must not stall boot or renewal.
    const AUTHORITY_TIMEOUT: Duration = Duration::from_secs(30);

    // The registry probes at intervals and reports after the last attempt, and the client
    // waits up to 45 s for that report.
    pub const PREFLIGHT_TIMEOUT: Duration = Duration::from_secs(60);

    // The registry lets a node ask for a preflight once in this long, counted from when it
    // admits the request. A retry sooner than that is refused as rate limited.
    pub const PREFLIGHT_BACKOFF_INITIAL: Duration = Duration::from_secs(15);

    /// How long to wait after the registry answers `RateLimited`. Never less than the
    /// registry's per-node window: a request refused for want of a free probe has still
    /// started that window.
    pub fn rate_limited_wait(retry_after_secs: u64) -> Duration {
        Duration::from_secs(retry_after_secs.max(1)).max(Self::PREFLIGHT_BACKOFF_INITIAL)
    }

    pub fn new(authority: PlayAuthority, conn: Arc<DatabaseConnection>, certs_path: &str) -> Self {
        Self {
            authority,
            conn,
            certs_path: certs_path.to_string(),
        }
    }

    pub fn new_shared(
        authority: PlayAuthority,
        conn: Arc<DatabaseConnection>,
        certs_path: &str,
    ) -> Arc<Self> {
        Arc::new(Self::new(authority, conn, certs_path))
    }

    /// The store play material for `name` lives in.
    pub fn storage_for(
        certs_path: &str,
        conn: Arc<DatabaseConnection>,
        name: &str,
    ) -> AcmeStorage {
        AcmeStorage::new_in_slot(
            certs_path,
            Self::STORAGE_DIR,
            Self::STORAGE_ROW,
            conn,
            Self::STORAGE_KEY.to_string(),
            vec![name.to_string()],
        )
    }

    /// A name and a usable certificate for it.
    ///
    /// When the registry no longer binds the stored name to this server, which happens after
    /// a month without a check-in, the stored name is forgotten and a new one is claimed. The
    /// registry answers a claim from a node that still holds a name with that same name, so
    /// the extra claim costs nothing in the ordinary case.
    pub async fn boot(
        &self,
        request: &PlayPreflightRequest,
        cancel: &CancellationToken,
    ) -> Result<(String, AcmeCertPaths)> {
        let mut backoff = PlayBackoff::new(Self::BOOT_BACKOFF_INITIAL, Self::BOOT_BACKOFF_CAP);
        let mut preflighted = false;
        loop {
            if !preflighted && self.needs_preflight().await? {
                self.preflight(request, cancel).await?;
                preflighted = true;
            }

            let name = self.claim_name(Some(request), cancel).await?;
            match self.ensure_certificate(&name, cancel).await {
                Ok(paths) => return Ok((name, paths)),
                Err(error) if Self::name_forgotten(&error) => {
                    PlayNameStore::clear(self.conn.as_ref()).await?;
                    let retry_in = backoff.next();
                    curia::warn!("Registry released the play name. Claiming a new one.", { "name": name, "retry_in": retry_in.as_secs() });
                    Self::wait(retry_in, cancel, "claiming a play name again").await?;
                }
                Err(error) => return Err(error),
            }
        }
    }

    /// True unless a stored name and a stored certificate that has not expired both exist.
    ///
    /// A server with both is already known to the registry and needs nothing new from it.
    /// Anything else is a request for a name or a certificate, and the registry hands
    /// neither to a server it cannot reach.
    pub async fn needs_preflight(&self) -> Result<bool> {
        let Some(name) = PlayNameStore::read(self.conn.as_ref()).await? else {
            return Ok(true);
        };

        Ok(match self.storage(&name).load_certificate_any().await? {
            Some(pem) => CertificateExpiry::is_expired(&pem).unwrap_or(true),
            None => true,
        })
    }

    /// Proves the registry can reach this server, and returns once it has.
    ///
    /// Binds `request.listen` to answer the registry's probe and releases it before
    /// returning, so the listeners can bind the same port. Retries with backoff until the
    /// probe passes or `cancel` fires: a failure here is the network, and the operator fixes
    /// it while this waits.
    pub async fn preflight(
        &self,
        request: &PlayPreflightRequest,
        cancel: &CancellationToken,
    ) -> Result<()> {
        let nonce = CurrentNonce::new_shared();
        let responder = PlayPreflightResponder::start(request.listen, nonce.clone()).await?;
        let result = self.preflight_until_reachable(request, &nonce, cancel).await;
        responder.stop().await;
        result
    }

    async fn preflight_until_reachable(
        &self,
        request: &PlayPreflightRequest,
        nonce: &CurrentNonce,
        cancel: &CancellationToken,
    ) -> Result<()> {
        let mut backoff =
            PlayBackoff::new(Self::PREFLIGHT_BACKOFF_INITIAL, Self::BOOT_BACKOFF_CAP);
        let mut gate = PreflightLogGate::new();
        let mut rate_limit_noted = false;
        let declared = request.address.is_some();
        loop {
            let timed = tokio::time::timeout(
                Self::PREFLIGHT_TIMEOUT,
                self.authority
                    .preflight(request.address.as_deref(), request.port, nonce),
            );
            let asked = tokio::select! {
                biased;
                _ = cancel.cancelled() => bail!("cancelled while asking for a play preflight"),
                asked = timed => asked,
            };

            let retry_in = match asked {
                Ok(Ok(outcome @ PlayPreflightOutcome::Reachable { .. })) => {
                    PlayPreflightReport::log_outcome(&outcome, request.quic_port, declared);
                    return Ok(());
                }
                Ok(Ok(outcome)) => {
                    let rate_limited = matches!(outcome, PlayPreflightOutcome::RateLimited { .. });
                    if gate.should_log(&outcome) || (rate_limited && !rate_limit_noted) {
                        rate_limit_noted |= rate_limited;
                        PlayPreflightReport::log_outcome(&outcome, request.quic_port, declared);
                    }
                    match outcome {
                        PlayPreflightOutcome::RateLimited { retry_after_secs } => {
                            Self::rate_limited_wait(retry_after_secs)
                        }
                        _ => backoff.next(),
                    }
                }
                Ok(Err(error)) => {
                    let retry_in = backoff.next();
                    Self::log_failure("Play preflight request failed. Retrying.", &error, retry_in);
                    retry_in
                }
                Err(_) => {
                    let retry_in = backoff.next();
                    Self::log_failure(
                        "Play preflight request failed. Retrying.",
                        &anyhow!("the registry did not answer within {:?}", Self::PREFLIGHT_TIMEOUT),
                        retry_in,
                    );
                    retry_in
                }
            };

            Self::wait(retry_in, cancel, "waiting for a play preflight").await?;
        }
    }

    /// The stored name, or a newly claimed one.
    ///
    /// Retries with backoff until the registry answers or `cancel` fires: a server that has
    /// no name has nothing to listen as.
    pub async fn name(&self, cancel: &CancellationToken) -> Result<String> {
        self.claim_name(None, cancel).await
    }

    // With a request, a claim the registry refuses for want of a passed preflight runs
    // preflight again: the registry keeps passes in memory and forgets them when it restarts.
    async fn claim_name(
        &self,
        request: Option<&PlayPreflightRequest>,
        cancel: &CancellationToken,
    ) -> Result<String> {
        if let Some(name) = PlayNameStore::read(self.conn.as_ref()).await? {
            return Ok(name);
        }

        let mut backoff = PlayBackoff::new(Self::BOOT_BACKOFF_INITIAL, Self::BOOT_BACKOFF_CAP);
        let mut preflight_reran = false;
        loop {
            let error = match tokio::time::timeout(Self::AUTHORITY_TIMEOUT, self.authority.claim())
                .await
            {
                Ok(Ok(name)) => {
                    PlayNameStore::write(self.conn.as_ref(), &name).await?;
                    curia::info!("Play name assigned", { "name": name.clone() });
                    return Ok(name);
                }
                Ok(Err(e)) => e,
                Err(_) => anyhow!("the registry did not answer within {:?}", Self::AUTHORITY_TIMEOUT),
            };

            if let Some(request) = request
                && Self::preflight_required(&error)
            {
                curia::info!("Registry has no passed play preflight. Running it again.");
                // The first repeat runs at once, because a restarted registry forgot the
                // pass. A repeat after that means the registry keeps refusing, so it waits.
                if preflight_reran {
                    let retry_in = backoff.next();
                    Self::wait(retry_in, cancel, "running the play preflight again").await?;
                }
                preflight_reran = true;
                self.preflight(request, cancel).await?;
                continue;
            }

            let retry_in = backoff.next();
            Self::log_failure("Play name claim failed. Retrying.", &error, retry_in);
            Self::wait(retry_in, cancel, "claiming a play name").await?;
        }
    }

    /// The paths of a certificate for `name` that has not expired, issuing one first when
    /// there is none.
    ///
    /// A certificate that expired while the server was stopped is replaced before this
    /// returns, so no listener ever starts on it. One that is past its renewal point but
    /// still valid is replaced if the registry answers, and served as it is if not.
    ///
    /// Fails at once when the registry says it holds no play name for this server, so the
    /// caller can claim again rather than retry a request that cannot succeed.
    pub async fn ensure_certificate(
        &self,
        name: &str,
        cancel: &CancellationToken,
    ) -> Result<AcmeCertPaths> {
        let storage = self.storage(name);

        match storage.load_certificate_any().await? {
            Some(pem) => match CertificateExpiry::is_expired(&pem) {
                Ok(false) => {
                    if !Self::due_for_renewal(&pem) {
                        return Ok(Self::paths(&storage));
                    }

                    curia::info!("Stored play certificate is due for renewal. Requesting a new one.", { "name": name });
                    return match self.issue(name).await {
                        Ok(()) => Ok(Self::paths(&storage)),
                        Err(error) if Self::name_forgotten(&error) => Err(error),
                        Err(error) => {
                            curia::warn!("Play certificate renewal failed. Serving the stored one.", { "error": format!("{error:#}"), "name": name });
                            Ok(Self::paths(&storage))
                        }
                    };
                }
                Ok(true) => {
                    curia::info!("Stored play certificate expired. Requesting a new one.", { "name": name });
                }
                Err(e) => {
                    curia::warn!("Stored play certificate is unreadable. Requesting a new one.", { "error": e.to_string(), "name": name });
                }
            },
            None => {
                curia::info!("No play certificate is stored. Requesting one.", { "name": name });
            }
        }

        let mut backoff = PlayBackoff::new(Self::BOOT_BACKOFF_INITIAL, Self::BOOT_BACKOFF_CAP);
        loop {
            match self.issue(name).await {
                Ok(()) => return Ok(Self::paths(&storage)),
                Err(error) if Self::name_forgotten(&error) => return Err(error),
                Err(error) => {
                    let retry_in = backoff.next();
                    Self::log_failure("Play certificate request failed. Retrying.", &error, retry_in);
                    Self::wait(retry_in, cancel, "obtaining a play certificate").await?;
                }
            }
        }
    }

    /// Requests a certificate for `name` with a new key, checks what comes back, and stores
    /// both.
    ///
    /// The registry is trusted to sign, not to be right: the leaf must name exactly `name`
    /// and carry the key generated here, or nothing is stored and the previous certificate
    /// stays in place.
    pub async fn issue(&self, name: &str) -> Result<()> {
        let key = KeyPair::generate_for(&PKCS_ECDSA_P256_SHA256)
            .context("generating the certificate key")?;

        let mut params = CertificateParams::new(vec![name.to_string()])
            .context("building the certificate request")?;
        params.distinguished_name = DistinguishedName::new();
        let request = params
            .serialize_request(&key)
            .context("signing the certificate request")?;

        let chain_pem = tokio::time::timeout(
            Self::AUTHORITY_TIMEOUT,
            self.authority.issue(request.der().to_vec()),
        )
        .await
        .map_err(|_| anyhow!("the registry did not answer within {:?}", Self::AUTHORITY_TIMEOUT))??;

        Self::check_leaf(&chain_pem, name, &key)?;

        let storage = self.storage(name);
        // The store files a certificate under an account row that ACME creates first. Play
        // has no account, so its own row is created empty. ACME never reads this row.
        if storage.load_account_credentials().await?.is_none() {
            storage.store_account_credentials("{}").await?;
        }
        storage
            .store_certificate(&chain_pem, &key.serialize_pem())
            .await?;

        curia::info!("Play certificate saved", { "name": name, "expires": Self::expires(&chain_pem) });
        Ok(())
    }

    // When the leaf expires, for the log. A chain that does not parse was rejected before
    // this is called, so the fallback is never seen in practice.
    fn expires(chain_pem: &str) -> String {
        use x509_parser::prelude::*;

        x509_parser::pem::parse_x509_pem(chain_pem.as_bytes())
            .ok()
            .and_then(|(_, pem)| {
                X509Certificate::from_der(&pem.contents)
                    .ok()
                    .map(|(_, leaf)| leaf.validity().not_after.to_datetime().to_string())
            })
            .unwrap_or_default()
    }

    /// Renews the certificate two thirds of the way through its life and signals `renewed_tx`
    /// each time, so the runtime can bounce the listeners that read it.
    ///
    /// A failed renewal is retried with backoff until it succeeds. The certificate in use
    /// stays in place meanwhile. If the registry has released the name, the stored name is
    /// forgotten so the next start claims a new one: the listeners cannot change name while
    /// running, so the operator is told to restart.
    pub fn spawn_renewal(
        self: Arc<Self>,
        name: String,
        cancel: CancellationToken,
        renewed_tx: mpsc::Sender<()>,
    ) -> JoinHandle<()> {
        tokio::spawn(async move {
            loop {
                let wait = match self.renewal_wait(&name).await {
                    Ok(wait) => wait,
                    Err(e) => {
                        curia::warn!("Play certificate renewal time is unreadable. Renewing now.", { "error": format!("{e:#}") });
                        Duration::ZERO
                    }
                };
                let wait = wait.max(Self::MIN_RENEWAL_GAP);

                curia::info!("Play certificate renewal scheduled", { "name": name.clone(), "in_hours": wait.as_secs() / 3600 });
                if Self::wait(wait, &cancel, "waiting to renew").await.is_err() {
                    return;
                }

                let mut backoff =
                    PlayBackoff::new(Self::RENEWAL_BACKOFF_INITIAL, Self::RENEWAL_BACKOFF_CAP);
                let mut name_cleared = false;
                loop {
                    match self.issue(&name).await {
                        Ok(()) => break,
                        Err(error) => {
                            let retry_in = backoff.next();
                            if Self::name_forgotten(&error) && !name_cleared {
                                name_cleared = true;
                                if let Err(e) = PlayNameStore::clear(self.conn.as_ref()).await {
                                    curia::warn!("Released play name could not be forgotten", { "error": e.to_string() });
                                }
                                curia::error!("Registry no longer holds the play name. Restart the server to claim a new one.", { "name": name.clone() });
                            } else {
                                Self::log_failure("Play certificate renewal failed. Retrying.", &error, retry_in);
                            }
                            if Self::wait(retry_in, &cancel, "retrying a renewal").await.is_err() {
                                return;
                            }
                        }
                    }
                }

                curia::info!("Play certificate renewed", { "name": name.clone() });
                tokio::select! {
                    biased;
                    _ = cancel.cancelled() => return,
                    sent = renewed_tx.send(()) => {
                        if sent.is_err() {
                            return;
                        }
                    }
                }
            }
        })
    }

    // How long until the stored certificate is due for renewal; zero when it already is.
    async fn renewal_wait(&self, name: &str) -> Result<Duration> {
        let pem = self
            .storage(name)
            .load_certificate_any()
            .await?
            .ok_or_else(|| anyhow!("no play certificate is stored"))?;
        let at = CertificateExpiry::renew_at(&pem)?;

        Ok(at
            .duration_since(SystemTime::now())
            .unwrap_or(Duration::ZERO))
    }

    // True when the renewal point has passed. An unreadable certificate counts as not due:
    // the caller has already decided it is unexpired.
    fn due_for_renewal(pem: &str) -> bool {
        CertificateExpiry::renew_at(pem).is_ok_and(|at| at <= SystemTime::now())
    }

    // The registry answered, and said it has no passed preflight for this server.
    fn preflight_required(error: &anyhow::Error) -> bool {
        error
            .downcast_ref::<EnrollmentError>()
            .and_then(EnrollmentError::reason)
            == Some(EnrollRefuseReason::PreflightRequired)
    }

    // The registry answered, and said it holds no play name for this server's key.
    fn name_forgotten(error: &anyhow::Error) -> bool {
        error
            .downcast_ref::<EnrollmentError>()
            .and_then(EnrollmentError::reason)
            == Some(EnrollRefuseReason::NoPlayName)
    }

    // A cause that retrying does not fix: the registry is too old, or does not offer play.
    fn permanent_cause(error: &anyhow::Error) -> Option<&'static str> {
        match error.downcast_ref::<EnrollmentError>()? {
            EnrollmentError::PlayNotOffered { .. } => {
                Some("Registry is older than enrollment version 3 and has no play names. Upgrade the registry.")
            }
            EnrollmentError::Refused {
                reason: EnrollRefuseReason::PlayUnavailable,
                ..
            } => Some("Registry does not offer play names."),
            _ => None,
        }
    }

    // A failure that will not clear by itself is an error naming its cause. The retry stays
    // slow either way, so the server comes up on its own once the registry changes.
    fn log_failure(message: &str, error: &anyhow::Error, retry_in: Duration) {
        match Self::permanent_cause(error) {
            Some(cause) => {
                curia::error!(message, { "cause": cause, "error": format!("{error:#}"), "retry_in": retry_in.as_secs() });
            }
            None => {
                curia::warn!(message, { "error": format!("{error:#}"), "retry_in": retry_in.as_secs() });
            }
        }
    }

    fn storage(&self, name: &str) -> AcmeStorage {
        Self::storage_for(&self.certs_path, self.conn.clone(), name)
    }

    fn paths(storage: &AcmeStorage) -> AcmeCertPaths {
        AcmeCertPaths {
            certificate: storage.certificate_path().to_string_lossy().into_owned(),
            key: storage.key_path().to_string_lossy().into_owned(),
        }
    }

    // Sleeps, or fails as soon as `cancel` fires.
    async fn wait(duration: Duration, cancel: &CancellationToken, what: &str) -> Result<()> {
        tokio::select! {
            biased;
            _ = cancel.cancelled() => bail!("cancelled while {what}"),
            _ = tokio::time::sleep(duration) => Ok(()),
        }
    }

    // The first certificate of the chain must name `name` and nothing else, and must be for
    // the key this server generated.
    fn check_leaf(chain_pem: &str, name: &str, key: &KeyPair) -> Result<()> {
        use x509_parser::extensions::GeneralName;
        use x509_parser::prelude::*;

        let (_, pem) = x509_parser::pem::parse_x509_pem(chain_pem.as_bytes())
            .map_err(|e| anyhow!("the registry's chain does not parse: {e}"))?;
        let (_, leaf) = X509Certificate::from_der(&pem.contents)
            .map_err(|e| anyhow!("the registry's leaf does not parse: {e}"))?;

        let names = leaf
            .subject_alternative_name()
            .map_err(|e| anyhow!("the registry's leaf has a malformed SAN: {e}"))?
            .map(|extension| extension.value.general_names.clone())
            .unwrap_or_default();

        let only_the_name = names.len() == 1
            && matches!(
                &names[0],
                GeneralName::DNSName(dns) if dns.eq_ignore_ascii_case(name)
            );
        if !only_the_name {
            bail!("the registry signed a certificate for other names than {name}");
        }

        if leaf.public_key().subject_public_key.data.as_ref() != key.public_key_raw() {
            bail!("the registry signed a certificate for a different key than the one requested");
        }

        Ok(())
    }
}
