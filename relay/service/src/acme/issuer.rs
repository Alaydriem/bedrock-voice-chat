use std::sync::Arc;
use std::time::Duration;

use common::curia;
use tokio_util::sync::CancellationToken;

use crate::config::AcmeConfig;
use crate::storage::{CertificateMaterial, CertificateStore};

use super::cloudflare_dns::CloudflareDns;
use super::error::AcmeError;
use super::propagation::PropagationCheck;

// The registry's own certificate, for one name.
//
// Deliberately much smaller than the server's `AcmeService`: one name, one provider,
// no storage abstraction, no legacy import. What they share is the DNS-01 shape, and
// duplicating that is cheaper than making one implementation serve two crates with
// different needs.
pub struct CertificateIssuer {
    config: AcmeConfig,
    hostname: String,
    store: Arc<CertificateStore>,
    dns: CloudflareDns,
    propagation: PropagationCheck,
}

impl CertificateIssuer {
    pub const RENEWAL_WINDOW_DAYS: i64 = 30;

    const CHECK_INTERVAL: Duration = Duration::from_secs(86_400);

    pub fn new(config: AcmeConfig, hostname: String, store: Arc<CertificateStore>) -> Self {
        let dns = CloudflareDns::new(&config.api_token);
        Self {
            config,
            hostname,
            store,
            dns,
            propagation: PropagationCheck::new(),
        }
    }

    // Issues only when the database holds no certificate for this hostname, or the one
    // it holds is inside its renewal window. Every issuance draws on a rate limit shared
    // with every assigned name, so one that is not needed is one an operator cannot
    // have.
    pub async fn ensure(&self) -> Result<CertificateMaterial, AcmeError> {
        if let Some(material) = self.current().await? {
            return Ok(material);
        }

        curia::info!("obtaining the registry certificate", { "hostname": self.hostname.clone() });
        self.issue().await
    }

    // The stored certificate, if there is one and it is not due for renewal.
    //
    // `None` means an issuance is owed, and the three reasons for it — absent, expiring,
    // unparseable — are deliberately not distinguished: the answer is the same, and a
    // certificate this process itself wrote but cannot read is one to replace rather
    // than one to refuse to start on.
    pub async fn current(&self) -> Result<Option<CertificateMaterial>, AcmeError> {
        let Some(material) = self.store.read(&self.hostname).await? else {
            return Ok(None);
        };

        if Self::expires_within(&material.chain_pem, Self::RENEWAL_WINDOW_DAYS) {
            return Ok(None);
        }

        Ok(Some(material))
    }

    // The renewal carries the new material rather than a bare signal. The listener
    // reloads from what it is handed, so there is no window where it re-reads storage
    // and finds the previous certificate still there.
    pub fn spawn_renewal(
        self: Arc<Self>,
        cancel: CancellationToken,
        renewed: tokio::sync::mpsc::Sender<CertificateMaterial>,
    ) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = cancel.cancelled() => break,
                    _ = tokio::time::sleep(Self::CHECK_INTERVAL) => {
                        match self.current().await {
                            Ok(Some(_)) => continue,
                            Ok(None) => {}
                            Err(e) => {
                                curia::error!(format!("reading the stored certificate failed; will retry: {e}"));
                                continue;
                            }
                        }

                        match self.issue().await {
                            Ok(material) => {
                                let _ = renewed.send(material).await;
                            }
                            Err(e) => curia::error!(
                                format!("renewing the registry certificate failed; will retry: {e}")
                            ),
                        }
                    }
                }
            }
        })
    }

    // A certificate that cannot be parsed is treated as one that needs replacing. The
    // alternative is refusing to start on material the process itself wrote.
    fn expires_within(pem: &str, days: i64) -> bool {
        let Some(der) = pem
            .split("-----BEGIN CERTIFICATE-----")
            .nth(1)
            .and_then(|body| body.split("-----END CERTIFICATE-----").next())
        else {
            return true;
        };

        use base64::Engine;
        let cleaned: String = der.chars().filter(|c| !c.is_whitespace()).collect();
        let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(cleaned) else {
            return true;
        };

        let Ok((_, parsed)) = x509_parser::parse_x509_certificate(&bytes) else {
            return true;
        };

        let remaining = parsed.validity().not_after.timestamp()
            - std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or_default();

        remaining < days * 86_400
    }

    async fn issue(&self) -> Result<CertificateMaterial, AcmeError> {
        let (account, _credentials) = instant_acme::Account::builder()
            .map_err(|e| AcmeError::Acme(e.to_string()))?
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
            .map_err(|e| AcmeError::Acme(e.to_string()))?;

        let identifiers = [instant_acme::Identifier::Dns(self.hostname.clone())];
        let mut order = account
            .new_order(&instant_acme::NewOrder::new(&identifiers))
            .await
            .map_err(|e| AcmeError::Acme(e.to_string()))?;

        let mut authorizations = order.authorizations();
        while let Some(result) = authorizations.next().await {
            let mut authz = result.map_err(|e| AcmeError::Acme(e.to_string()))?;
            if !matches!(authz.status, instant_acme::AuthorizationStatus::Pending) {
                continue;
            }

            let mut challenge = authz
                .challenge(instant_acme::ChallengeType::Dns01)
                .ok_or_else(|| AcmeError::Acme("no DNS-01 challenge offered".to_string()))?;
            let domain = challenge.identifier().to_string();
            let value = challenge.key_authorization().dns_value();

            self.dns.publish_txt(&domain, &value).await?;
            self.propagation
                .wait_for(&CloudflareDns::challenge_name(&domain), &value)
                .await?;
            challenge
                .set_ready()
                .await
                .map_err(|e| AcmeError::Acme(e.to_string()))?;
        }
        drop(authorizations);

        let retry = instant_acme::RetryPolicy::default();
        order
            .poll_ready(&retry)
            .await
            .map_err(|e| AcmeError::Acme(e.to_string()))?;
        let key_pem = order
            .finalize()
            .await
            .map_err(|e| AcmeError::Acme(e.to_string()))?;
        let chain_pem = order
            .poll_certificate(&retry)
            .await
            .map_err(|e| AcmeError::Acme(e.to_string()))?;

        let material = CertificateMaterial::new(chain_pem, key_pem);
        self.store.write(&self.hostname, &material).await?;

        // Best effort. A challenge left behind is untidy rather than harmful, and
        // failing the issuance over it would waste the order that just succeeded.
        if let Err(e) = self.dns.cleanup_txt(&self.hostname).await {
            curia::warn!(format!("could not clean up the challenge record: {e}"));
        }

        curia::info!("registry certificate issued", { "hostname": self.hostname.clone() });
        Ok(material)
    }
}
