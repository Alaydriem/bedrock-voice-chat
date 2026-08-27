use std::net::{IpAddr, SocketAddr};

use anyhow::{Context, Result};
use axum_server::tls_rustls::RustlsConfig;
use common::curia;
use tokio_util::sync::CancellationToken;

use crate::config::HttpConfig;
use crate::storage::CertificateMaterial;
use crate::runtime::TlsProvider;

// The operator-facing surface, over TLS and nothing else.
//
// There is no plain-HTTP path here and no setting that introduces one: this is where
// enrollment tokens are handed out, and serving them unencrypted would put them on the
// network in clear.
pub struct HttpServer;

impl HttpServer {
    // The listener, built rather than delegated.
    //
    // Binding `[::]` and hoping is not portable: Windows defaults `IPV6_V6ONLY` to on,
    // so a v6 wildcard there refuses every IPv4 client — including a `curl` against a
    // `127.0.0.1` hosts entry, which fails as a connection refused that names nothing.
    // Linux defaults it off, which is why the same code appears to work there.
    //
    // Clearing the flag explicitly makes both platforms behave the same way. It is only
    // possible because `axum_server::from_tcp_rustls` accepts a socket configured here;
    // the BVC server cannot do the same because Rocket binds internally.
    fn listener(bind: IpAddr, port: u16) -> Result<std::net::TcpListener> {
        use socket2::{Domain, Protocol, Socket, Type};

        let domain = match bind {
            IpAddr::V4(_) => Domain::IPV4,
            IpAddr::V6(_) => Domain::IPV6,
        };
        let socket = Socket::new(domain, Type::STREAM, Some(Protocol::TCP))
            .context("creating the registry HTTP socket")?;

        // Only the v6 wildcard. A specific v6 address is a request for that interface,
        // and an operator who wrote `0.0.0.0` asked for IPv4 alone — neither should be
        // widened underneath them.
        if matches!(bind, IpAddr::V6(v6) if v6.is_unspecified()) {
            // Best effort. A host that refuses to clear it leaves an IPv6-only
            // listener, which is worth a line in the log rather than a failure to start.
            if let Err(e) = socket.set_only_v6(false) {
                curia::warn!(format!(
                    "this host refuses a dual-stack socket; IPv4 clients will not reach the registry: {e}"
                ));
            }
        }

        socket
            .set_reuse_address(true)
            .context("setting SO_REUSEADDR")?;

        let addr = SocketAddr::from((bind, port));
        socket.bind(&addr.into()).with_context(|| {
            format!("binding the registry HTTP listener on {addr}; is the port already in use?")
        })?;
        socket.listen(1024).context("listening")?;

        let listener: std::net::TcpListener = socket.into();
        listener
            .set_nonblocking(true)
            .context("setting the listener non-blocking")?;

        Ok(listener)
    }

    // The handle is supplied rather than made here so the caller can observe the bound
    // address. With `port = 0` that is the only way to learn which port was taken.
    pub async fn serve(
        config: HttpConfig,
        router: axum::Router,
        material: CertificateMaterial,
        renewed: tokio::sync::mpsc::Receiver<CertificateMaterial>,
        cancel: CancellationToken,
        handle: axum_server::Handle,
    ) -> Result<()> {
        TlsProvider::install();

        // From memory, never from a file. The certificate lives in the database and is
        // handed here as bytes, so the container needs no volume and there is no second
        // copy on disk that can disagree with the stored one.
        let rustls = RustlsConfig::from_pem(
            material.chain_pem.into_bytes(),
            material.key_pem.into_bytes(),
        )
        .await
        .context("loading the registry certificate")?;

        Self::spawn_reload(rustls.clone(), renewed, cancel.clone());

        let bind = config.bind_address().map_err(|e| anyhow::anyhow!(e))?;
        let listener = Self::listener(bind, config.port)?;
        let addr = listener
            .local_addr()
            .context("reading the bound address")?;
        curia::info!("registry HTTP listening", { "bind": addr.to_string(), "hostname": config.hostname.clone() });

        let shutdown = handle.clone();
        tokio::spawn(async move {
            cancel.cancelled().await;
            shutdown.graceful_shutdown(Some(std::time::Duration::from_secs(5)));
        });

        axum_server::from_tcp_rustls(listener, rustls)
            .handle(handle)
            .serve(router.into_make_service())
            .await
            .context("serving the registry HTTP surface")
    }

    // Reloads in place rather than restarting the listener. A renewal every sixty days
    // is not worth dropping connections for, and a restart would need the port back
    // before anything else took it.
    //
    // The channel carries the new material rather than a bare signal, so the reload
    // uses exactly what was just issued instead of re-reading storage and racing it.
    pub fn spawn_reload(
        rustls: RustlsConfig,
        mut renewed: tokio::sync::mpsc::Receiver<CertificateMaterial>,
        cancel: CancellationToken,
    ) {
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = cancel.cancelled() => break,
                    message = renewed.recv() => {
                        let Some(material) = message else {
                            break;
                        };
                        match rustls
                            .reload_from_pem(
                                material.chain_pem.into_bytes(),
                                material.key_pem.into_bytes(),
                            )
                            .await
                        {
                            Ok(()) => curia::info!("reloaded the renewed registry certificate"),
                            Err(e) => curia::error!(format!(
                                "could not reload the renewed certificate: {e}"
                            )),
                        }
                    }
                }
            }
        });
    }
}
