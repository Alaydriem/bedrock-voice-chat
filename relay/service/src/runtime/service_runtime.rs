use std::sync::Arc;

use anyhow::{Context, Result};
use bvc_relay::node::NodeIdentity;
use common::curia;
use tokio_util::sync::CancellationToken;

use crate::acme::CertificateIssuer;
use crate::budget::WeeklyBudget;
use crate::config::RelayConfig;
use crate::db::Db;
use crate::discord::{DiscordBotClient, DiscordOAuthClient, IdentitySource, MemberSource};
use crate::dns::{CloudflareApi, CloudflareClient, ZoneWriter};
use crate::http::{HttpServer, HttpState, Router as HttpRouter};
use crate::registry::ClaimService;
use crate::registry::RegistryEndpoint;
use crate::registry::RegistryService;
use crate::runtime::TlsProvider;
use crate::scheduler::DailyScheduler;
use crate::storage::{CertificateMaterial, CertificateStore, NodeKeyStore};
use crate::validation::{AddressProbe, LiveAddressProbe, ValidationChecker};

// The registry.
//
// An iroh endpoint that assigns hostnames to entitled members and tells any server
// the address it was seen at. It relays nothing: two peers that cannot reach each
// other directly do not connect, and no traffic is ever carried through this host.
pub struct ServiceRuntime {
    config: RelayConfig,
}

impl ServiceRuntime {
    pub fn new(config: RelayConfig) -> Self {
        Self { config }
    }

    pub async fn start(&self) -> Result<()> {
        // Before the first reqwest client, not before the listener: building a client
        // constructs a TLS config, so the Discord and Cloudflare clients below reach
        // rustls long before anything serves.
        TlsProvider::install();

        let conn = Arc::new(Db::connect(&self.config.database_url).await?);

        // The registry's identity, from the database. Every enrolled server holds a
        // peer link naming this key, so it is durable state rather than something a
        // container may regenerate when its filesystem is replaced.
        let node_secret = NodeKeyStore::new_shared(conn.clone())
            .resolve()
            .await
            .context("resolving the relay registry node key")?;
        let identity = NodeIdentity::from_secret_bytes(&node_secret);

        let registry = RegistryService::new_shared(
            conn.clone(),
            self.config.discord.clone(),
            MemberSource::Bot(DiscordBotClient::new(&self.config.discord)),
        );

        let zone = Arc::new(ZoneWriter::new(
            conn.clone(),
            CloudflareApi::Live(CloudflareClient::new(&self.config.cloudflare)),
            self.config.zone.clone(),
        ));

        let budget = WeeklyBudget::new_shared(
            conn.clone(),
            self.config.weekly_certificate_ceiling,
        );

        let enroll = RegistryEndpoint::bind(
            &identity,
            registry.clone(),
            zone.clone(),
            budget,
            Some(self.config.enroll_port),
        )
        .await?;
        enroll.spawn_accept_loop();

        // The value an operator pastes into a server's `registry` block. Logged
        // at every start because it is minted from the live endpoint, not assembled
        // from configuration, and an operator has nowhere else to read it.
        curia::info!("relay registry enrollment link", { "peerlink": enroll.ticket().await?, "node": enroll.node_id().to_string() });

        let registry_for_http = Arc::clone(&registry);
        let checker = ValidationChecker::new_shared(conn.clone(), registry, zone);
        let scheduler = DailyScheduler::new_shared(
            conn.clone(),
            checker,
            Arc::clone(enroll.sessions()),
            MemberSource::Bot(DiscordBotClient::new(&self.config.discord)),
            self.config.discord.clone(),
            AddressProbe::Live(LiveAddressProbe::new()),
        );

        let cancel = CancellationToken::new();
        scheduler.spawn(cancel.clone());

        curia::info!("relay registry started", { "zone": self.config.zone.clone(), "enroll_port": self.config.enroll_port, "weekly_certificate_ceiling": self.config.weekly_certificate_ceiling });

        let issuer = Arc::new(CertificateIssuer::new(
            self.config
                .http
                .cloudflare()
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "http.acme \"cloudflare\" is required: the registry needs a certificate \
                         for its own hostname, and TLS is not optional"
                    )
                })?
                .clone(),
            self.config.http.hostname.clone(),
            CertificateStore::new_shared(conn.clone()),
        ));
        let material = issuer.ensure().await?;

        let (renewed_tx, renewed_rx) = tokio::sync::mpsc::channel::<CertificateMaterial>(1);
        issuer.clone().spawn_renewal(cancel.clone(), renewed_tx);

        let identity_source = IdentitySource::OAuth(DiscordOAuthClient::new(
            &self.config.discord,
            self.config.http.redirect_uri(),
        ));
        let http_state = HttpState::new_shared(
            self.config.http.clone(),
            self.config.discord.clone(),
            registry_for_http,
            ClaimService::new_shared(conn),
            identity_source,
        );
        let router = HttpRouter::build(http_state);

        let http = tokio::spawn(HttpServer::serve(
            self.config.http.clone(),
            router,
            material,
            renewed_rx,
            cancel.clone(),
            axum_server::Handle::new(),
        ));

        tokio::select! {
            result = tokio::signal::ctrl_c() => {
                result.context("waiting for a shutdown signal")?;
                curia::info!("shutting down");
            }
            result = http => {
                curia::error!("the registry HTTP surface stopped");
                cancel.cancel();
                result.context("the HTTP task panicked")??;
            }
        }

        cancel.cancel();

        Ok(())
    }
}
