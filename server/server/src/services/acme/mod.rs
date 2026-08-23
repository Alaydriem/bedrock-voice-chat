//! ACME DNS-01 certificate management: issuance, storage, and renewal.

mod cert_paths;
mod expiry;
mod propagation;
mod provider;
mod storage;

pub use cert_paths::AcmeCertPaths;
pub use expiry::CertificateExpiry;
pub use propagation::PropagationChecker;
pub use provider::{AcmeDnsProvider, CloudflareProvider, DnsProvider};
pub use storage::AcmeStorage;

use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result, anyhow};
use tokio_util::sync::CancellationToken;
use common::curia;

use crate::config::Acme;

// Renew when fewer than 30 days of validity remain; check daily.
const RENEWAL_WINDOW: Duration = Duration::from_secs(30 * 86400);
const RENEWAL_CHECK_INTERVAL: Duration = Duration::from_secs(86400);

/// Obtains and renews the HTTPS certificate via ACME DNS-01.
pub struct AcmeService {
    config: Acme,
    domains: Vec<String>,
    provider: DnsProvider,
    storage: AcmeStorage,
    propagation: PropagationChecker,
}

impl AcmeService {
    pub fn new(
        config: Acme,
        tls_names: &[String],
        certs_path: &str,
        conn: Arc<sea_orm::DatabaseConnection>,
    ) -> Result<Self> {
        config.validate(tls_names)?;
        let domains = config.effective_domains(tls_names)?;
        let provider = DnsProvider::from_config(&config)?;
        let storage = AcmeStorage::new(
            certs_path,
            conn,
            config.directory.clone(),
            domains.clone(),
        );
        Ok(Self {
            config,
            domains,
            provider,
            storage,
            propagation: PropagationChecker::new(),
        })
    }

    /// Returns the active certificate paths, issuing a fresh certificate
    /// first when none is stored or the stored one enters the renewal window.
    /// Startup issuance retries with backoff: transient DNS/CA hiccups at
    /// boot should not require operator intervention, but a persistent
    /// misconfiguration must still fail startup with the failing step named.
    pub async fn ensure_certificate(&self) -> Result<AcmeCertPaths> {
        // Adopts an account and certificate a pre-database deployment left on disk, before
        // anything decides to issue. Re-registering and re-issuing both cost quota with the
        // provider, and neither is recoverable by trying again.
        self.storage.import_legacy().await?;

        if self
            .storage
            .load_certificate_valid_for(RENEWAL_WINDOW)
            .await?
            .is_none()
        {
            curia::info!("No valid ACME certificate stored; issuing", { "domains": format!("{:?}", self.domains) });
            let mut last_error = None;
            for (attempt, delay_secs) in [(1u32, 30u64), (2, 60), (3, 0)] {
                match self.issue().await {
                    Ok(()) => {
                        last_error = None;
                        break;
                    }
                    Err(e) => {
                        curia::error!("ACME issuance attempt failed", { "error": e.to_string(), "attempt": attempt });
                        last_error = Some(e);
                        if delay_secs > 0 {
                            tokio::time::sleep(Duration::from_secs(delay_secs)).await;
                        }
                    }
                }
            }
            if let Some(e) = last_error {
                return Err(e.context("ACME issuance failed after 3 attempts"));
            }
        }
        Ok(AcmeCertPaths {
            certificate: self
                .storage
                .certificate_path()
                .to_string_lossy()
                .into_owned(),
            key: self.storage.key_path().to_string_lossy().into_owned(),
        })
    }

    /// True when a renewal was performed.
    pub async fn renew_if_needed(&self) -> Result<bool> {
        if self
            .storage
            .load_certificate_valid_for(RENEWAL_WINDOW)
            .await?
            .is_some()
        {
            return Ok(false);
        }
        curia::info!("ACME certificate entering renewal window; renewing", { "domains": format!("{:?}", self.domains) });
        self.issue().await?;
        Ok(true)
    }

    /// Daily renewal loop under the runtime's cancellation tree. Sends on
    /// `renewed_tx` after a successful renewal so the runtime can bounce the
    /// HTTP listener. Errors are logged and retried next tick — an
    /// unreachable CA today is not fatal while the current cert is valid.
    pub fn spawn_renewal(
        self: Arc<Self>,
        cancel: CancellationToken,
        renewed_tx: tokio::sync::mpsc::Sender<()>,
    ) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(RENEWAL_CHECK_INTERVAL);
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            // The first tick fires immediately; harmless, since issuance just
            // happened at startup.
            loop {
                tokio::select! {
                    _ = cancel.cancelled() => break,
                    _ = interval.tick() => {
                        match self.renew_if_needed().await {
                            Ok(true) => {
                                let _ = renewed_tx.send(()).await;
                            }
                            Ok(false) => {}
                            Err(e) => curia::error!("ACME renewal attempt failed; will retry", { "error": e.to_string() }),
                        }
                    }
                }
            }
        })
    }

    /// One full DNS-01 issuance: order, publish TXT per authorization, wait
    /// for propagation, validate, finalize, persist.
    async fn issue(&self) -> Result<()> {
        let account = self.load_or_create_account().await?;

        let identifiers: Vec<instant_acme::Identifier> = self
            .domains
            .iter()
            .map(|d| instant_acme::Identifier::Dns(d.clone()))
            .collect();
        let mut order = account
            .new_order(&instant_acme::NewOrder::new(&identifiers))
            .await
            .context("creating ACME order")?;

        // Publish a TXT record per pending authorization, confirm public
        // visibility, then mark the challenge ready. Sequential on purpose:
        // few domains, and it keeps borrow scopes simple.
        let mut published: Vec<String> = Vec::new();
        let mut authorizations = order.authorizations();
        while let Some(result) = authorizations.next().await {
            let mut authz = result.context("fetching ACME authorization")?;
            if !matches!(authz.status, instant_acme::AuthorizationStatus::Pending) {
                continue;
            }
            let mut challenge = authz
                .challenge(instant_acme::ChallengeType::Dns01)
                .ok_or_else(|| anyhow!("no DNS-01 challenge offered"))?;
            let domain = challenge.identifier().to_string();
            let txt_value = challenge.key_authorization().dns_value();
            let fqdn = format!("_acme-challenge.{domain}");

            self.provider
                .publish_txt(&domain, &txt_value)
                .await
                .with_context(|| format!("publishing TXT for {domain}"))?;
            published.push(domain.clone());
            self.propagation
                .wait_for_txt(&fqdn, &txt_value)
                .await
                .with_context(|| format!("waiting for TXT propagation for {domain}"))?;
            challenge
                .set_ready()
                .await
                .with_context(|| format!("marking challenge ready for {domain}"))?;
        }
        drop(authorizations);

        let retry = instant_acme::RetryPolicy::default();
        order
            .poll_ready(&retry)
            .await
            .context("waiting for order to become ready")?;
        let private_key_pem = order.finalize().await.context("finalizing ACME order")?;
        let cert_chain_pem = order
            .poll_certificate(&retry)
            .await
            .context("downloading certificate chain")?;

        self.storage
            .store_certificate(&cert_chain_pem, &private_key_pem)
            .await?;
        curia::info!("ACME certificate issued and stored", { "domains": format!("{:?}", self.domains) });

        for domain in published {
            if let Err(e) = self.provider.cleanup_txt(&domain).await {
                curia::error!("failed to clean up challenge TXT record", { "error": e.to_string(), "domain": domain });
            }
        }
        Ok(())
    }

    /// Reuses the persisted ACME account or registers a new one. The account
    /// key is generate-once, like ca.key.
    async fn load_or_create_account(&self) -> Result<instant_acme::Account> {
        if let Some(json) = self.storage.load_account_credentials().await? {
            let credentials: instant_acme::AccountCredentials =
                serde_json::from_str(&json).context("parsing stored ACME account credentials")?;
            return Ok(instant_acme::Account::builder()
                .context("building ACME client")?
                .from_credentials(credentials)
                .await
                .context("loading ACME account")?);
        }
        let (account, credentials) = instant_acme::Account::builder()
            .context("building ACME client")?
            .create(
                &instant_acme::NewAccount {
                    contact: &[&format!("mailto:{}", self.config.email)],
                    terms_of_service_agreed: true,
                    only_return_existing: false,
                },
                self.config.directory.clone(),
                None,
            )
            .await
            .context("creating ACME account")?;
        self.storage
            .store_account_credentials(
                &serde_json::to_string(&credentials)
                    .context("serializing ACME account credentials")?,
            )
            .await?;
        Ok(account)
    }
}
