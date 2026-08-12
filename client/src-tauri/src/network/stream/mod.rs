mod connect_outcome;
mod health_manager;
mod health_publisher;
pub(crate) mod link;
mod stream_manager;

pub use health_publisher::HealthPublisher;

use connect_outcome::{AttemptResult, ConnectOutcome};
use link::DatagramLink;

use crate::AudioPacket;
use crate::NetworkPacket;
use crate::diagnostics::{LinkSession, QuicLinkStats, QuicStatsSubscriber, TransportStats};
use common::net::CandidatePlan;
use common::net::NetTimeouts;
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
    /// Which servers have already proven QUIC unusable. Lives on the manager because the
    /// manager outlives every `restart`, which is the whole point: the verdict has to
    /// survive the reconnect that follows the failure that produced it.
    transport_verdict: Arc<crate::network::TransportVerdict>,
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
        // Built before the health manager, which records demotions into it.
        let transport_verdict = crate::network::TransportVerdict::new_shared();
        let health_manager =
            ConnectionHealthManager::new(app_handle.clone(), transport_verdict.clone());

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
            transport_verdict,
        }
    }

    /// The verdict store, so the health monitor can record a session that connected and
    /// then stopped carrying traffic.
    ///
    /// Public for the end-to-end harness, which pre-demotes a host to place one client on
    /// the WebSocket transport while another stays on QUIC. That drives the real selection
    /// branch rather than bypassing it: the state it writes is exactly the state a client
    /// reaches in the field after QUIC degrades on it.
    pub fn transport_verdict(&self) -> Arc<crate::network::TransportVerdict> {
        self.transport_verdict.clone()
    }

    /// Initializes a new network connection to the server, and immediately begins
    pub async fn restart(
        &mut self,
        server_fqdn: String,
        server_url: String,
        plan: CandidatePlan,
        // The reachability probe's verdict on UDP. When it is `false` the probe has
        // already spent a full handshake budget proving nothing answers, and walking the
        // same endpoints again would spend a second one on the player's clock.
        quic_reachable: bool,
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

        // A host that has already degraded a QUIC session skips the walk entirely. Probing
        // it again would say "reachable" — that is exactly the signal a degrading network
        // defeats — and the walk would then hand back a session that stops carrying audio a
        // minute later.
        let link = if self.transport_verdict.is_demoted(&server_fqdn) {
            log::info!("QUIC is demoted for {server_fqdn}; connecting over WebSocket");
            self.connect_websocket(&server_url, &ca_cert, &cert, &key)
                .await?
        } else if !quic_reachable {
            // The reachability probe already spent a full handshake budget establishing
            // that nothing answers on UDP. Walking the same endpoints again would spend a
            // second one rediscovering it, and the player waits through both.
            log::info!(
                "no QUIC endpoint answered the probe for {server_fqdn}; connecting over WebSocket"
            );
            self.transport_verdict.demote(&server_fqdn);
            self.connect_websocket(&server_url, &ca_cert, &cert, &key)
                .await?
        } else {
            // The probe said UDP answers, so QUIC is worth attempting — but a probe that
            // answered is not a session that carried. The two transports race from here,
            // and the loser costs nothing.
            let mut outcome = ConnectOutcome::new();
            let raced = self
                .race_transports(
                    &client,
                    &plan,
                    &server_fqdn,
                    &server_url,
                    &ca_cert,
                    &cert,
                    &key,
                    &mut outcome,
                )
                .await;
            self.report_connect_outcome(&outcome, &server_url);
            raced?
        };

        self.health_manager.reset();

        self.input = StreamTraitType::Input(stream_manager::InputStream::new(
            self.producer.clone(),
            Some(link.clone()),
            self.app_handle.clone(),
            self.health_manager.health_state(),
            self.transport_stats.clone(),
            self.quic_stats_tx.subscribe(),
        ));

        self.output = StreamTraitType::Output(stream_manager::OutputStream::new(
            self.consumer.clone(),
            identity.clone(),
            Some(link.clone()),
            self.app_handle.clone(),
            self.transport_stats.clone(),
        ));

        self.input.start().await?;
        self.output.start().await?;
        self.health_manager.start(link, server_url);

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

    /// Runs both transports against each other and returns whichever carried a session.
    ///
    /// QUIC is given a head start, so on any path where it works the WebSocket attempt is
    /// never made: the delay expires after QUIC has already won. That matters on the
    /// server, which would otherwise perform a TLS handshake and register a session for a
    /// link nobody ends up using, once per connecting player.
    ///
    /// When the WebSocket attempt does win, QUIC keeps running for the remainder of its
    /// handshake budget and takes the session if it lands. Preferring the faster answer
    /// unconditionally would move a merely distant player onto TCP for being distant.
    #[allow(clippy::too_many_arguments)]
    async fn race_transports(
        &self,
        client: &Client,
        plan: &CandidatePlan,
        server_fqdn: &str,
        server_url: &str,
        ca_cert: &str,
        cert: &str,
        key: &str,
        outcome: &mut ConnectOutcome,
    ) -> Result<DatagramLink, String> {
        // Rendered to a String at the source rather than at each use: the walk's error is a
        // `Box<dyn Error>`, which is not Send, and the race below holds the pending result
        // of one transport across the other's await.
        let quic = async {
            Self::connect_first_available(client, plan, server_fqdn, outcome)
                .await
                .map_err(|e| e.to_string())
        };
        tokio::pin!(quic);

        let websocket = async {
            tokio::time::sleep(NetTimeouts::WEBSOCKET_HEAD_START).await;
            self.connect_websocket(server_url, ca_cert, cert, key).await
        };
        tokio::pin!(websocket);

        tokio::select! {
            // Biased so a QUIC handshake landing in the same instant as the WebSocket one
            // is still preferred, rather than left to scheduler order.
            biased;

            attempt = &mut quic => {
                match attempt {
                    Ok((connection, winner)) => {
                        self.adopt_quic(connection, winner, server_url, ca_cert)
                    }
                    // Every candidate failed inside the head start. The API answered — this
                    // client fetched its configuration over it — so TCP reaches the server
                    // and only UDP does not.
                    Err(detail) => {
                        log::warn!(
                            "no QUIC candidate carried a session ({detail}); using WebSocket"
                        );
                        self.transport_verdict.demote(server_fqdn);
                        Ok(websocket.await?)
                    }
                }
            }

            link = &mut websocket => {
                let link = link?;
                log::info!(
                    "the WebSocket transport connected first; giving QUIC {:?} to overtake",
                    NetTimeouts::QUIC_OVERTAKE
                );

                match tokio::time::timeout(NetTimeouts::QUIC_OVERTAKE, &mut quic).await {
                    Ok(Ok((connection, winner))) => {
                        log::info!("QUIC overtook the WebSocket transport; using QUIC");
                        self.adopt_quic(connection, winner, server_url, ca_cert)
                    }
                    // QUIC failed outright while an alternative was already in hand. That
                    // is evidence, so it is recorded for the rest of the run.
                    Ok(Err(detail)) => {
                        log::warn!(
                            "no QUIC candidate carried a session ({detail}); using WebSocket"
                        );
                        self.transport_verdict.demote(server_fqdn);
                        Ok(link)
                    }
                    // QUIC neither succeeded nor failed inside its budget. Slow is not the
                    // same as broken, so this session runs on WebSocket and the next one
                    // races again from scratch — no verdict is recorded.
                    Err(_) => {
                        log::warn!(
                            "QUIC did not answer within {:?} while the WebSocket transport was ready; using WebSocket for this session",
                            NetTimeouts::QUIC_OVERTAKE
                        );
                        Ok(link)
                    }
                }
            }
        }
    }

    /// Records the winning QUIC candidate as the live session and wraps it as the link.
    fn adopt_quic(
        &self,
        mut connection: Connection,
        winner: ConnectCandidate,
        server_url: &str,
        ca_cert: &str,
    ) -> Result<DatagramLink, String> {
        connection.keep_alive(true).map_err(|e| e.to_string())?;

        // Family comes from the winning candidate, never from its dial address: on a
        // dual-stack socket an IPv4 destination is dialed as `::ffff:a.b.c.d`, so
        // classifying the address would report every dual-stack client as IPv6.
        self.link_session.set(
            Some(winner.family()),
            winner.port(),
            common::structs::metrics::TransportKind::Quic,
            server_url.to_string(),
            ca_cert,
        );

        Ok(DatagramLink::Quic(Arc::new(connection)))
    }

    /// Opens the WebSocket voice transport against the same public port the API uses.
    ///
    /// There is no separate host, port or capability flag to fetch: the server's TLS
    /// demultiplexer routes by ALPN, so `wss://` on the API's own authority reaches the
    /// voice listener. A server too old to have a demultiplexer offers no such protocol
    /// and this fails fast rather than hanging.
    async fn connect_websocket(
        &self,
        server_url: &str,
        ca_cert: &str,
        cert: &str,
        key: &str,
    ) -> Result<DatagramLink, String> {
        let authority = server_url
            .strip_prefix("https://")
            .or_else(|| server_url.strip_prefix("http://"))
            .unwrap_or(server_url)
            .trim_end_matches('/');
        let url = format!("wss://{authority}/voice");

        log::info!("Connecting the WebSocket voice transport to {url}");
        let link = link::WsLink::connect(&url, ca_cert, cert, key)
            .await
            .map_err(|e| e.to_string())?;
        log::info!("WebSocket voice transport connected");

        // The family is deliberately absent: the TLS dialler picks it, and reporting a
        // guess would be worse than reporting nothing. The port is the public one, which
        // is the only port this transport ever uses.
        self.link_session.set(
            None,
            Self::authority_port(authority),
            common::structs::metrics::TransportKind::WebSocket,
            server_url.to_string(),
            ca_cert,
        );

        Ok(DatagramLink::WebSocket(link))
    }

    // The port a server authority names, or the HTTPS default when it names none.
    fn authority_port(authority: &str) -> u16 {
        let after_host = match authority.rfind(']') {
            Some(end) => &authority[end + 1..],
            None => authority,
        };

        after_host
            .rsplit_once(':')
            .and_then(|(_, port)| port.parse().ok())
            .unwrap_or(443)
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

        // Defaults negotiate a 30s idle timeout and derive the keepalive from it at 3/4, so a
        // ping reaches the wire every 22.5s. Carrier translators routinely drop an idle UDP
        // mapping sooner; it is then recreated on a new source port, which the server sees as
        // a new path. s2n-quic allows five and reclaims none, so the fifth rebinding silently
        // drops every datagram after it — a session that connects, works, and quietly stops
        // carrying audio without either end reporting an error.
        //
        // Inert unless `Connection::keep_alive` is enabled; `restart` enables it on the
        // connection this client produces, so removing that call reverts this silently.
        let limits = common::s2n_quic::provider::limits::Limits::default()
            .with_max_keep_alive_period(std::time::Duration::from_secs(10))?
            .with_max_idle_timeout(std::time::Duration::from_secs(45))?;

        // The tracing subscriber stays in the tuple. `with_event` replaces the default event
        // provider outright, so dropping it here would silently remove every QUIC trace with
        // nothing failing to indicate it.
        let client = Client::builder()
            .with_tls(provider)?
            .with_io(bind)?
            .with_limits(limits)?
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
    // Emitted whether the walk succeeded or not. A network that blocks UDP outright and one
    // that reaches the server on an alternate port both end the walk; only the per-candidate
    // record separates them, and neither is visible from the error code alone.
    fn report_connect_outcome(&self, outcome: &ConnectOutcome, server_url: &str) {
        use tauri::Manager;

        if let Some(analytics) = self
            .app_handle
            .try_state::<std::sync::Arc<crate::analytics::AnalyticsService>>()
        {
            analytics.track(
                common::structs::AnalyticsEvent::VoiceTransportOutcome,
                Some(outcome.properties(server_url)),
            );
        }
    }

    // `outcome` accumulates every attempt, including the successful one, so the caller can
    // report the whole walk rather than only its verdict. Filled on both the success and the
    // failure path: a walk that reached the server on its third candidate is as diagnostic as
    // one that reached nothing.
    async fn connect_first_available(
        client: &Client,
        plan: &CandidatePlan,
        server_fqdn: &str,
        outcome: &mut ConnectOutcome,
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
                    outcome.record(*candidate, AttemptResult::Connected);
                    return Ok((connection, *candidate));
                }
                Ok(Err(e)) => {
                    log::warn!(
                        "QUIC handshake rejected on {} ({:?}): {}",
                        candidate.dial(),
                        candidate.family(),
                        e
                    );
                    outcome.record(*candidate, AttemptResult::Rejected);
                    last_error = Some(e.to_string());
                }
                Err(_) => {
                    log::warn!(
                        "QUIC handshake timed out on {} ({:?}) after {:?}",
                        candidate.dial(),
                        candidate.family(),
                        candidate.budget()
                    );
                    outcome.record(*candidate, AttemptResult::TimedOut);
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
