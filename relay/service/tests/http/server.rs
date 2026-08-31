use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use axum_server::Handle;
use bvc_relay_service::config::{DiscordConfig, HttpConfig};
use bvc_relay_service::db::Db;
use bvc_relay_service::discord::{FixedMemberSource, IdentitySource, MemberSource};
use bvc_relay_service::http::{HttpServer, HttpState, Router};
use bvc_relay_service::registry::{ClaimService, RegistryService};
use bvc_relay_service::storage::CertificateMaterial;
use time::Duration as Validity;
use tokio::sync::mpsc::Sender;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use crate::harness::CertificateFixture;

const SERVED_NAME: &str = "first.test";
const RENEWED_NAME: &str = "second.test";

// A live server on an ephemeral port, plus the levers a test needs to drive it.
struct Running {
    addr: SocketAddr,
    ca_pem: String,
    cancel: CancellationToken,
    renewed: Sender<CertificateMaterial>,
    served: JoinHandle<anyhow::Result<()>>,
}

impl Running {
    // Port 0, so a test never collides with anything else on the machine and never has
    // to guess which port is free. The bound port is read back from the handle, which
    // is why the handle is injected rather than made inside `serve`.
    async fn start(name: &str) -> Self {
        let fixture = CertificateFixture::issue(name, Validity::days(30));

        let (renewed, renewed_rx) = tokio::sync::mpsc::channel::<CertificateMaterial>(1);
        let cancel = CancellationToken::new();
        let handle = Handle::new();

        let mut served = tokio::spawn(HttpServer::serve(
            Self::config(name),
            Self::router().await,
            fixture.material,
            renewed_rx,
            cancel.clone(),
            handle.clone(),
        ));

        // Raced against the server task rather than simply awaited. A `serve` that
        // fails before it binds never notifies the handle, so awaiting `listening`
        // alone would hang here instead of reporting why it failed.
        let addr = tokio::time::timeout(Duration::from_secs(20), async {
            tokio::select! {
                listening = handle.listening() => {
                    listening.expect("the server binds its listener")
                }
                stopped = &mut served => {
                    panic!("the server stopped before binding: {stopped:?}")
                }
            }
        })
        .await
        .expect("the server binds within twenty seconds");

        Self {
            addr,
            ca_pem: fixture.ca_pem,
            cancel,
            renewed,
            served,
        }
    }

    // The bind address is a parameter so a test can assert what each value produces,
    // rather than asserting the default twice.
    async fn start_bound(name: &str, bind: &str) -> Self {
        let fixture = CertificateFixture::issue(name, Validity::days(30));
        let (renewed, renewed_rx) = tokio::sync::mpsc::channel::<CertificateMaterial>(1);
        let cancel = CancellationToken::new();
        let handle = Handle::new();

        let mut config = Self::config(name);
        config.bind = bind.to_string();

        let mut served = tokio::spawn(HttpServer::serve(
            config,
            Self::router().await,
            fixture.material,
            renewed_rx,
            cancel.clone(),
            handle.clone(),
        ));

        let addr = tokio::time::timeout(Duration::from_secs(20), async {
            tokio::select! {
                listening = handle.listening() => listening.expect("the server binds"),
                stopped = &mut served => panic!("the server stopped before binding: {stopped:?}"),
            }
        })
        .await
        .expect("the server binds within twenty seconds");

        Self {
            addr,
            ca_pem: fixture.ca_pem,
            cancel,
            renewed,
            served,
        }
    }

    fn config(name: &str) -> HttpConfig {
        HttpConfig {
            hostname: name.to_string(),
            page_origin: "https://page.example".to_string(),
            port: 0,
            bind: "::".to_string(),
            acme: Default::default(),
        }
    }

    async fn router() -> axum::Router {
        let conn = Arc::new(Db::connect("sqlite::memory:").await.expect("connects"));
        let discord = DiscordConfig {
            guild_id: "guild".to_string(),
            bot_token: "bot".to_string(),
            client_id: "client".to_string(),
            client_secret: "secret".to_string(),
            qualifying_role_ids: vec!["role-a".to_string()],
        };
        let registry = RegistryService::new_shared(
            conn.clone(),
            discord.clone(),
            MemberSource::Fixed(FixedMemberSource::absent()),
        );
        let state = HttpState::new_shared(
            Self::config(SERVED_NAME),
            discord,
            registry,
            ClaimService::new_shared(conn),
            IdentitySource::Fixed("member-1".to_string()),
        );
        Router::build(state)
    }

    // A client that trusts exactly one CA and resolves `name` to this server's port.
    // Trusting one certificate rather than disabling verification is what makes the
    // handshake itself an assertion.
    fn client(&self, trusting: &str, name: &str) -> reqwest::Client {
        reqwest::Client::builder()
            .add_root_certificate(
                reqwest::Certificate::from_pem(trusting.as_bytes()).expect("a PEM certificate"),
            )
            .resolve(name, self.connect_addr())
            .build()
            .expect("a client")
    }

    // The handle reports the bind address, which is the wildcard. A client has to be
    // pointed at a loopback on that port instead — the wildcard is not connectable.
    fn connect_addr(&self) -> SocketAddr {
        SocketAddr::from((std::net::Ipv6Addr::LOCALHOST, self.addr.port()))
    }

    fn client_at(&self, ip: std::net::IpAddr) -> reqwest::Client {
        reqwest::Client::builder()
            .add_root_certificate(
                reqwest::Certificate::from_pem(self.ca_pem.as_bytes()).expect("a PEM certificate"),
            )
            .resolve(SERVED_NAME, SocketAddr::from((ip, self.addr.port())))
            .timeout(Duration::from_secs(5))
            .build()
            .expect("a client")
    }

    fn url(&self, name: &str) -> String {
        format!("https://{name}:{}/healthz", self.addr.port())
    }

}

// The whole TLS path under a real socket: bind, handshake, route, respond. Nothing
// else exercises rustls, and a certificate the server cannot load fails only here.
#[tokio::test]
async fn the_server_answers_over_tls() {
    let running = Running::start(SERVED_NAME).await;

    let response = running
        .client(&running.ca_pem, SERVED_NAME)
        .get(running.url(SERVED_NAME))
        .send()
        .await
        .expect("a TLS response");

    assert_eq!(response.status(), reqwest::StatusCode::OK);

    running.cancel.cancel();
}

// A renewal must reach the live listener. Without the reload the server keeps
// presenting the old certificate until something restarts it — which is the failure
// this mechanism exists to prevent, and it surfaces sixty days after anyone last
// looked.
#[tokio::test]
async fn a_renewed_certificate_is_served_without_a_restart() {
    let running = Running::start(SERVED_NAME).await;

    let renewed = CertificateFixture::issue(RENEWED_NAME, Validity::days(90));
    let asking_for_the_new_name = running.client(&renewed.ca_pem, RENEWED_NAME);
    let url = running.url(RENEWED_NAME);

    // Before the reload the server still presents the old certificate, so a client
    // asking for the new name must fail. Without this the test would pass against a
    // reload that never happened.
    assert!(
        asking_for_the_new_name.get(&url).send().await.is_err(),
        "the pre-reload handshake must fail on the certificate's name"
    );

    // The channel carries the material itself, so nothing has to reach storage or a
    // file for the reload to take effect.
    running
        .renewed
        .send(renewed.material.clone())
        .await
        .expect("signals a renewal");

    let mut served_the_renewal = false;
    for _ in 0..50 {
        if asking_for_the_new_name.get(&url).send().await.is_ok() {
            served_the_renewal = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    assert!(served_the_renewal, "the renewed certificate was never served");

    running.cancel.cancel();
}

// The listener has to answer IPv4 clients, not only IPv6 ones.
//
// Windows defaults `IPV6_V6ONLY` to on, so a plain `[::]` bind there refuses every
// IPv4 client — a `curl` against a `127.0.0.1` hosts entry included. It fails as a
// connection refused that names nothing, and it looks identical to a server that never
// started. Linux defaults the flag off, so this only ever breaks on some machines.
#[tokio::test]
async fn an_ipv4_client_reaches_the_listener() {
    let running = Running::start(SERVED_NAME).await;

    let over_v4 = reqwest::Client::builder()
        .add_root_certificate(
            reqwest::Certificate::from_pem(running.ca_pem.as_bytes()).expect("a PEM certificate"),
        )
        .resolve(
            SERVED_NAME,
            SocketAddr::from((std::net::Ipv4Addr::LOCALHOST, running.addr.port())),
        )
        .build()
        .expect("a client");

    let response = over_v4
        .get(running.url(SERVED_NAME))
        .send()
        .await
        .expect("an IPv4 client reaches the dual-stack listener");

    assert_eq!(response.status(), reqwest::StatusCode::OK);

    running.cancel.cancel();
}

// `http.bind` is the override, and `0.0.0.0` has to mean IPv4 alone. Widening it back
// to dual-stack would ignore an operator who deliberately kept the listener off IPv6.
#[tokio::test]
async fn binding_ipv4_only_serves_ipv4_and_not_ipv6() {
    let running = Running::start_bound(SERVED_NAME, "0.0.0.0").await;

    let over_v4 = running.client_at(std::net::Ipv4Addr::LOCALHOST.into());
    assert_eq!(
        over_v4
            .get(running.url(SERVED_NAME))
            .send()
            .await
            .expect("IPv4 reaches an IPv4 listener")
            .status(),
        reqwest::StatusCode::OK
    );

    let over_v6 = running.client_at(std::net::Ipv6Addr::LOCALHOST.into());
    assert!(
        over_v6.get(running.url(SERVED_NAME)).send().await.is_err(),
        "an IPv4-only listener must not answer IPv6"
    );

    running.cancel.cancel();
}

// A typo has to stop the start naming the value. Falling through to the socket call
// would fail with an address-family error that names neither the field nor what was in
// it.
#[tokio::test]
async fn an_unparseable_bind_address_is_refused_by_name() {
    let mut config = Running::config(SERVED_NAME);
    config.bind = "not-an-address".to_string();

    let error = config.bind_address().expect_err("refuses");

    assert!(error.contains("http.bind"));
    assert!(error.contains("not-an-address"));
}

// Cancellation must end the server rather than leave the task running. A listener that
// outlives its shutdown holds the port, and the next start fails to bind for a reason
// that names the port rather than the cause.
#[tokio::test]
async fn cancelling_stops_the_server() {
    let running = Running::start(SERVED_NAME).await;

    running.cancel.cancel();

    let stopped = tokio::time::timeout(Duration::from_secs(15), running.served)
        .await
        .expect("the server stops within the grace window")
        .expect("the server task does not panic");

    assert!(stopped.is_ok(), "a cancelled server stops cleanly");
}
