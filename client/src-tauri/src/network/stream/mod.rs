mod health_manager;
mod stream_manager;

use crate::AudioPacket;
use crate::NetworkPacket;
use crate::diagnostics::{LinkSession, QuicLinkStats, QuicStatsSubscriber, TransportStats};
use common::net::CandidatePlan;
use common::net::ConnectCandidate;
use common::s2n_quic::Client;
use common::s2n_quic::Connection;
use common::s2n_quic::client::Connect;
use std::error::Error;
use std::sync::Arc;
use stream_manager::StreamTrait;
use stream_manager::StreamTraitType;
use tauri::Manager;
use tokio::sync::watch;

use health_manager::ConnectionHealthManager;

pub(crate) struct NetworkStreamManager {
    producer: Arc<flume::Sender<AudioPacket>>,
    consumer: Arc<flume::Receiver<NetworkPacket>>,
    input: StreamTraitType,
    output: StreamTraitType,
    app_handle: tauri::AppHandle,
    health_manager: ConnectionHealthManager,
    quic_stats_tx: watch::Sender<Arc<QuicLinkStats>>,
    link_session: Arc<LinkSession>,
    transport_stats: Arc<TransportStats>,
}

impl NetworkStreamManager {
    /// Initializes the NetworkStreamManager
    /// By default, this doesn't do anything accept setup the StreamTraitTypes
    /// The stream will not start until it is connected
    pub fn new(
        producer: Arc<flume::Sender<AudioPacket>>,
        consumer: Arc<flume::Receiver<NetworkPacket>>,
        app_handle: tauri::AppHandle,
        quic_stats_tx: watch::Sender<Arc<QuicLinkStats>>,
        link_session: Arc<LinkSession>,
        transport_stats: Arc<TransportStats>,
    ) -> Self {
        let health_manager = ConnectionHealthManager::new(app_handle.clone());

        Self {
            producer: producer.clone(),
            consumer: consumer.clone(),
            input: StreamTraitType::Input(stream_manager::InputStream::new(
                producer.clone(),
                None,
                app_handle.clone(),
                health_manager.health_state(),
                transport_stats.clone(),
                quic_stats_tx.subscribe(),
            )),
            output: StreamTraitType::Output(stream_manager::OutputStream::new(
                consumer.clone(),
                String::new(),
                None,
                app_handle.clone(),
                transport_stats.clone(),
            )),
            app_handle: app_handle.clone(),
            health_manager,
            quic_stats_tx,
            link_session,
            transport_stats,
        }
    }

    /// Initializes a new network connection to the server, and immediately begins
    pub async fn restart(
        &mut self,
        server_fqdn: String,
        server_url: String,
        plan: CandidatePlan,
        identity: String,
        ca_cert: String,
        cert: String,
        key: String,
    ) -> Result<(), Box<dyn Error>> {
        self.stop().await?;

        // A client with no IPv6 candidates stays on the plain IPv4 socket that every
        // released version uses. Only a client that actually intends to try IPv6
        // takes the dual-stack path, where IPv4 destinations travel v4-mapped.
        //
        // A failed dual-stack bind means the host has IPv6 unbound entirely. The v6
        // candidates are undialable from the replacement socket, so they are dropped
        // along with it rather than left in the walk to time out.
        let (client, plan) = if plan.requires_v6_socket() {
            // The bind error is rendered to a String before the retry: it is a
            // `Box<dyn Error>`, which is not Send, and holding one across the next
            // await would make every caller's future non-Send.
            let first = Self::build_client(
                "[::]:0",
                &ca_cert,
                &cert,
                &key,
                self.quic_stats_tx.clone(),
            )
            .await
            .map_err(|e| e.to_string());

            match first {
                Ok(client) => (client, plan),
                Err(detail) => {
                    log::warn!(
                        "Dual-stack QUIC socket bind failed ({}); retrying on IPv4 only",
                        detail
                    );
                    // The abandoned endpoint never accepted a connection, so it never minted a
                    // stats context and published nothing.
                    let client = Self::build_client(
                        "0.0.0.0:0",
                        &ca_cert,
                        &cert,
                        &key,
                        self.quic_stats_tx.clone(),
                    )
                    .await?;
                    (client, plan.without_ipv6())
                }
            }
        } else {
            let client = Self::build_client(
                "0.0.0.0:0",
                &ca_cert,
                &cert,
                &key,
                self.quic_stats_tx.clone(),
            )
            .await?;
            (client, plan)
        };

        let (mut connection, winner) =
            Self::connect_first_available(&client, &plan, &server_fqdn).await?;
        connection.keep_alive(true)?;

        // Family comes from the winning candidate, never from its dial address: on a
        // dual-stack socket an IPv4 destination is dialed as `::ffff:a.b.c.d`, so classifying
        // the address would report every dual-stack client as IPv6.
        self.link_session.set(
            winner.family(),
            winner.port(),
            server_url.clone(),
            &ca_cert,
        );
        let conn_arc = Arc::new(connection);
        self.health_manager.reset();

        self.input = StreamTraitType::Input(stream_manager::InputStream::new(
            self.producer.clone(),
            Some(conn_arc.clone()),
            self.app_handle.clone(),
            self.health_manager.health_state(),
            self.transport_stats.clone(),
            self.quic_stats_tx.subscribe(),
        ));

        self.output = StreamTraitType::Output(stream_manager::OutputStream::new(
            self.consumer.clone(),
            identity.clone(),
            Some(conn_arc.clone()),
            self.app_handle.clone(),
            self.transport_stats.clone(),
        ));

        self.input.start().await?;
        self.output.start().await?;
        self.health_manager.start(conn_arc, server_url);

        // The control plane reports under the same identity the server authenticated this
        // connection as, because that is what the server compares every report against.
        // Publish it, then nudge a full snapshot so a fresh player's state is never empty.
        if let Some(connection_identity) = self
            .app_handle
            .try_state::<Arc<crate::control::ConnectionIdentity>>()
        {
            connection_identity.set(Some(identity));
        }
        if let Some(bus) = self.app_handle.try_state::<crate::control::ControlStateBus>() {
            bus.self_state();
            bus.preferences();
        }

        Ok(())
    }

    // Builds an endpoint bound to `bind`. The mTLS provider and the datagram
    // endpoint are each consumed by the builder and neither is Clone, so every
    // attempt constructs its own rather than sharing.
    async fn build_client(
        bind: &str,
        ca_cert: &str,
        cert: &str,
        key: &str,
        stats_tx: watch::Sender<Arc<QuicLinkStats>>,
    ) -> Result<Client, Box<dyn Error>> {
        let provider = common::rustls::MtlsProvider::new_from_vec(
            ca_cert.as_bytes().to_vec(),
            cert.as_bytes().to_vec(),
            key.as_bytes().to_vec(),
        )
        .await?;

        let dg_endpoint = common::s2n_quic::provider::datagram::default::Endpoint::builder()
            .with_send_capacity(1024)
            .expect("send cap > 0")
            .with_recv_capacity(1024)
            .expect("recv cap > 0")
            .build()
            .expect("build dg endpoint");

        // The tracing subscriber stays in the tuple. `with_event` replaces the default event
        // provider outright, so dropping it here would silently remove every QUIC trace with
        // nothing failing to indicate it.
        let client = Client::builder()
            .with_tls(provider)?
            .with_io(bind)?
            .with_datagram(dg_endpoint)?
            .with_event((
                common::s2n_quic::provider::event::tracing::Subscriber::default(),
                QuicStatsSubscriber::new(stats_tx),
            ))?
            .start()?;

        Ok(client)
    }

    // Walks the plan in order and returns the first completed handshake. The winning
    // family is logged alongside the winning port: that split is the only field
    // signal separating "IPv6 works for this player" from "IPv6 was tried and IPv4
    // carried the session".
    //
    // A blackholed UDP port produces no response at all, so the per-candidate
    // timeout — not an error — is what ends an attempt and moves on.
    // The winning candidate is returned alongside the connection rather than recorded from
    // inside the walk, which keeps this a pure function of its inputs and leaves exactly one
    // place that decides what the current session is.
    async fn connect_first_available(
        client: &Client,
        plan: &CandidatePlan,
        server_fqdn: &str,
    ) -> Result<(Connection, ConnectCandidate), Box<dyn Error>> {
        let mut last_error: Option<String> = None;

        for candidate in plan.candidates() {
            let connect = Connect::new(candidate.dial()).with_server_name(server_fqdn.to_string());

            match tokio::time::timeout(candidate.budget(), client.connect(connect)).await {
                Ok(Ok(connection)) => {
                    log::info!(
                        "QUIC handshake succeeded on {} ({:?}, port {})",
                        candidate.dial(),
                        candidate.family(),
                        candidate.port()
                    );
                    return Ok((connection, *candidate));
                }
                Ok(Err(e)) => {
                    log::warn!(
                        "QUIC handshake rejected on {} ({:?}): {}",
                        candidate.dial(),
                        candidate.family(),
                        e
                    );
                    last_error = Some(e.to_string());
                }
                Err(_) => {
                    log::warn!(
                        "QUIC handshake timed out on {} ({:?}) after {:?}",
                        candidate.dial(),
                        candidate.family(),
                        candidate.budget()
                    );
                    last_error = Some(format!("timed out after {:?}", candidate.budget()));
                }
            }
        }

        Err(last_error
            .unwrap_or_else(|| "no candidate QUIC endpoints were available".to_string())
            .into())
    }

    pub async fn stop(&mut self) -> Result<(), anyhow::Error> {
        self.clear_connection_identity();
        self.health_manager.stop();
        self.input.stop().await?;
        self.output.stop().await?;

        Ok(())
    }

    // A stopped stream must not keep reporting as the old connection; the
    // reporter skips reports while no identity is published.
    //
    // The link session is cleared alongside it, so a disconnected client stops publishing a
    // port, a family, and an uptime that climbs against a connection that no longer exists.
    fn clear_connection_identity(&self) {
        self.link_session.clear();

        if let Some(identity) = self
            .app_handle
            .try_state::<Arc<crate::control::ConnectionIdentity>>()
        {
            identity.set(None);
        }
    }

    /// Discards audio captured for a connection that is going away, returning how much went.
    ///
    /// Nothing else empties this queue: the sending end is an `Arc` clone held by whichever
    /// input stream the audio manager currently has, and rebuilding that stream does not touch
    /// what it already sent.
    pub fn drain_outbound(&self) -> usize {
        self.consumer.drain().count()
    }

    pub async fn reset(&mut self) -> Result<(), anyhow::Error> {
        self.clear_connection_identity();
        self.health_manager.stop();
        let (_, _) = tokio::join!(self.input.stop(), self.output.stop());
        self.health_manager.reset();

        self.input = StreamTraitType::Input(stream_manager::InputStream::new(
            self.producer.clone(),
            None,
            self.app_handle.clone(),
            self.health_manager.health_state(),
            self.transport_stats.clone(),
            self.quic_stats_tx.subscribe(),
        ));

        self.output = StreamTraitType::Output(stream_manager::OutputStream::new(
            self.consumer.clone(),
            String::new(),
            None,
            self.app_handle.clone(),
            self.transport_stats.clone(),
        ));

        Ok(())
    }
}
