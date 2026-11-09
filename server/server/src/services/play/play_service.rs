use std::sync::Arc;
use std::time::{Duration, SystemTime};

use anyhow::{Context, Result, anyhow, bail};
use common::curia;
use common::structs::relay::enroll::EnrollRefuseReason;
use rcgen::{CertificateParams, DistinguishedName, KeyPair, PKCS_ECDSA_P256_SHA256};
use sea_orm::DatabaseConnection;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use super::{PlayAuthority, PlayBackoff, PlayNameStore};
use crate::services::EnrollmentError;
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
    pub async fn boot(&self, cancel: &CancellationToken) -> Result<(String, AcmeCertPaths)> {
        let mut backoff = PlayBackoff::new(Self::BOOT_BACKOFF_INITIAL, Self::BOOT_BACKOFF_CAP);
        loop {
            let name = self.name(cancel).await?;
            match self.ensure_certificate(&name, cancel).await {
                Ok(paths) => return Ok((name, paths)),
                Err(error) if Self::name_forgotten(&error) => {
                    PlayNameStore::clear(self.conn.as_ref()).await?;
                    let retry_in = backoff.next();
                    curia::warn!("the registry holds no play name for this server; claiming again", { "name": name, "retry_in": retry_in.as_secs() });
                    Self::wait(retry_in, cancel, "claiming a play name again").await?;
                }
                Err(error) => return Err(error),
            }
        }
    }

    /// The stored name, or a newly claimed one.
    ///
    /// Retries with backoff until the registry answers or `cancel` fires: a server that has
    /// no name has nothing to listen as.
    pub async fn name(&self, cancel: &CancellationToken) -> Result<String> {
        if let Some(name) = PlayNameStore::read(self.conn.as_ref()).await? {
            return Ok(name);
        }

        let mut backoff = PlayBackoff::new(Self::BOOT_BACKOFF_INITIAL, Self::BOOT_BACKOFF_CAP);
        loop {
            let error = match tokio::time::timeout(Self::AUTHORITY_TIMEOUT, self.authority.claim())
                .await
            {
                Ok(Ok(name)) => {
                    PlayNameStore::write(self.conn.as_ref(), &name).await?;
                    curia::info!("this server claimed a play name", { "name": name.clone() });
                    return Ok(name);
                }
                Ok(Err(e)) => e,
                Err(_) => anyhow!("the registry did not answer within {:?}", Self::AUTHORITY_TIMEOUT),
            };

            let retry_in = backoff.next();
            Self::log_failure("could not claim a play name; retrying", &error, retry_in);
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

                    curia::info!("the stored play certificate is due for renewal; requesting a new one", { "name": name });
                    return match self.issue(name).await {
                        Ok(()) => Ok(Self::paths(&storage)),
                        Err(error) if Self::name_forgotten(&error) => Err(error),
                        Err(error) => {
                            curia::warn!("could not renew the play certificate; serving the stored one", { "error": format!("{error:#}"), "name": name });
                            Ok(Self::paths(&storage))
                        }
                    };
                }
                Ok(true) => {
                    curia::info!("the stored play certificate has expired; requesting a new one", { "name": name });
                }
                Err(e) => {
                    curia::warn!("the stored play certificate cannot be read; requesting a new one", { "error": e.to_string(), "name": name });
                }
            },
            None => {
                curia::info!("no play certificate is stored; requesting one", { "name": name });
            }
        }

        let mut backoff = PlayBackoff::new(Self::BOOT_BACKOFF_INITIAL, Self::BOOT_BACKOFF_CAP);
        loop {
            match self.issue(name).await {
                Ok(()) => return Ok(Self::paths(&storage)),
                Err(error) if Self::name_forgotten(&error) => return Err(error),
                Err(error) => {
                    let retry_in = backoff.next();
                    Self::log_failure("could not obtain a play certificate; retrying", &error, retry_in);
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

        curia::info!("play certificate stored", { "name": name });
        Ok(())
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
                        curia::warn!("could not read the play certificate's renewal time; renewing", { "error": format!("{e:#}") });
                        Duration::ZERO
                    }
                };
                let wait = wait.max(Self::MIN_RENEWAL_GAP);

                curia::info!("play certificate renewal scheduled", { "name": name.clone(), "in_hours": wait.as_secs() / 3600 });
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
                                    curia::warn!("could not forget the released play name", { "error": e.to_string() });
                                }
                                curia::error!("the registry no longer holds this server's play name; restart the server to claim a new one", { "name": name.clone() });
                            } else {
                                Self::log_failure("play certificate renewal failed; retrying", &error, retry_in);
                            }
                            if Self::wait(retry_in, &cancel, "retrying a renewal").await.is_err() {
                                return;
                            }
                        }
                    }
                }

                curia::info!("play certificate renewed", { "name": name.clone() });
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
                Some("the registry is older than enrollment version 3 and has no play names; it must be upgraded")
            }
            EnrollmentError::Refused {
                reason: EnrollRefuseReason::PlayUnavailable,
                ..
            } => Some("the registry does not offer play names"),
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
