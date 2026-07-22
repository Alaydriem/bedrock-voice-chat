use crate::config::BedrockDnsConfig;
use common::traits::StreamTrait;
use moka::sync::Cache;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;
use tokio::net::{TcpListener, UdpSocket};
use tokio::sync::watch;
use tokio::task::AbortHandle;

use super::RateLimitEntry;

pub struct DnsService {
    config: BedrockDnsConfig,
    lan_ip: IpAddr,
    abort_handle: Option<AbortHandle>,
    shutdown_tx: Option<watch::Sender<bool>>,
}

impl DnsService {
    pub fn new(config: BedrockDnsConfig, lan_ip: IpAddr) -> Self {
        Self {
            config,
            lan_ip,
            abort_handle: None,
            shutdown_tx: None,
        }
    }

    fn build_rate_limiter() -> Cache<IpAddr, Arc<RateLimitEntry>> {
        Cache::builder()
            .time_to_live(Duration::from_secs(1))
            .max_capacity(10_000)
            .build()
    }

    fn check_rate_limit(
        limiter: &Cache<IpAddr, Arc<RateLimitEntry>>,
        src_ip: IpAddr,
        limit: u32,
    ) -> bool {
        let entry = limiter.get_with(src_ip, || {
            Arc::new(RateLimitEntry {
                count: AtomicU32::new(0),
            })
        });
        let prev = entry.count.fetch_add(1, Ordering::Relaxed);
        prev < limit
    }

    async fn start_server_loop(
        &self,
        shutdown_rx: watch::Receiver<bool>,
    ) -> Result<AbortHandle, anyhow::Error> {
        let override_host = self.config.override_host.clone();
        let lan_ip = self.lan_ip;
        let rate_limit = self.config.rate_limit_per_sec;
        let upstream_addrs: Vec<SocketAddr> = self
            .config
            .upstream
            .iter()
            .filter_map(|s| format!("{}:53", s).parse().ok())
            .collect();

        let bind_addr: SocketAddr = format!("0.0.0.0:{}", self.config.port).parse()?;

        let udp_socket = Arc::new(UdpSocket::bind(bind_addr).await?);
        tracing::info!("Bedrock DNS server listening on UDP {}", bind_addr);

        let tcp_listener = TcpListener::bind(bind_addr).await?;
        tracing::info!("Bedrock DNS server listening on TCP {}", bind_addr);

        let override_host_udp = override_host.clone();
        let upstream_udp = upstream_addrs.clone();
        let shutdown_rx_udp = shutdown_rx.clone();
        let shutdown_rx_tcp = shutdown_rx.clone();

        let handle = tokio::spawn(async move {
            tokio::spawn(Self::serve_udp(
                udp_socket,
                override_host_udp,
                lan_ip,
                upstream_udp,
                rate_limit,
                shutdown_rx_udp,
            ));

            tokio::spawn(Self::serve_tcp(
                tcp_listener,
                override_host,
                lan_ip,
                upstream_addrs,
                rate_limit,
                shutdown_rx_tcp,
            ));

            let mut rx = shutdown_rx;
            let _ = rx.wait_for(|&v| v).await;
            tracing::info!("DNS service shutdown signal received");
        });

        Ok(handle.abort_handle())
    }

    async fn serve_udp(
        socket: Arc<UdpSocket>,
        override_host: String,
        lan_ip: IpAddr,
        upstream: Vec<SocketAddr>,
        rate_limit: u32,
        mut shutdown_rx: watch::Receiver<bool>,
    ) {
        let rate_limiter = Self::build_rate_limiter();
        let mut buf = vec![0u8; 4096];
        loop {
            tokio::select! {
                result = socket.recv_from(&mut buf) => {
                    let (len, src) = match result {
                        Ok(r) => r,
                        Err(e) => {
                            tracing::error!("DNS UDP recv error: {}", e);
                            continue;
                        }
                    };

                    if !Self::check_rate_limit(&rate_limiter, src.ip(), rate_limit) {
                        tracing::warn!("DNS rate limit exceeded for {}", src.ip());
                        continue;
                    }

                    let query = buf[..len].to_vec();
                    let override_host = override_host.clone();
                    let upstream = upstream.clone();
                    let socket = Arc::clone(&socket);

                    tokio::spawn(async move {
                        match Self::handle_dns_query(&query, &override_host, lan_ip, &upstream).await {
                            Ok(response) => {
                                if let Err(e) = socket.send_to(&response, src).await {
                                    tracing::error!("DNS UDP send error: {}", e);
                                }
                            }
                            Err(e) => {
                                tracing::error!("DNS query handling error: {}", e);
                            }
                        }
                    });
                }
                _ = shutdown_rx.wait_for(|&v| v) => {
                    break;
                }
            }
        }
    }

    async fn serve_tcp(
        listener: TcpListener,
        override_host: String,
        lan_ip: IpAddr,
        upstream: Vec<SocketAddr>,
        rate_limit: u32,
        mut shutdown_rx: watch::Receiver<bool>,
    ) {
        let rate_limiter = Self::build_rate_limiter();
        loop {
            tokio::select! {
                result = listener.accept() => {
                    let (mut stream, addr) = match result {
                        Ok(r) => r,
                        Err(e) => {
                            tracing::error!("DNS TCP accept error: {}", e);
                            continue;
                        }
                    };

                    if !Self::check_rate_limit(&rate_limiter, addr.ip(), rate_limit) {
                        tracing::warn!("DNS TCP rate limit exceeded for {}", addr.ip());
                        continue;
                    }

                    let override_host = override_host.clone();
                    let upstream = upstream.clone();

                    tokio::spawn(async move {
                        use tokio::io::{AsyncReadExt, AsyncWriteExt};
                        let mut len_buf = [0u8; 2];
                        if stream.read_exact(&mut len_buf).await.is_err() {
                            return;
                        }
                        let msg_len = u16::from_be_bytes(len_buf) as usize;
                        let mut query = vec![0u8; msg_len];
                        if stream.read_exact(&mut query).await.is_err() {
                            return;
                        }

                        match Self::handle_dns_query(&query, &override_host, lan_ip, &upstream).await {
                            Ok(response) => {
                                let len_bytes = (response.len() as u16).to_be_bytes();
                                let _ = stream.write_all(&len_bytes).await;
                                let _ = stream.write_all(&response).await;
                            }
                            Err(e) => {
                                tracing::error!("DNS TCP query error: {}", e);
                            }
                        }
                    });
                }
                _ = shutdown_rx.wait_for(|&v| v) => {
                    break;
                }
            }
        }
    }

    async fn handle_dns_query(
        query: &[u8],
        override_host: &str,
        lan_ip: IpAddr,
        upstream: &[SocketAddr],
    ) -> Result<Vec<u8>, anyhow::Error> {
        use hickory_proto::op::Message;
        use hickory_proto::serialize::binary::BinDecodable;

        let message = Message::from_bytes(query)?;

        let is_override = message.queries.iter().any(|q| {
            let name = q.name().to_ascii().trim_end_matches('.').to_lowercase();
            name == override_host
        });

        if is_override {
            Self::build_override_response(&message, lan_ip)
        } else {
            Self::forward_to_upstream(query, upstream).await
        }
    }

    fn build_override_response(
        query: &hickory_proto::op::Message,
        lan_ip: IpAddr,
    ) -> Result<Vec<u8>, anyhow::Error> {
        use hickory_proto::op::{Message, MessageType, Metadata, OpCode, ResponseCode};
        use hickory_proto::rr::rdata::{A, AAAA};
        use hickory_proto::rr::{Name, RData, Record, RecordType};
        use std::str::FromStr;

        let mut response = Message::new(query.metadata.id, MessageType::Response, OpCode::Query);
        let mut metadata = Metadata::response_from_request(&query.metadata);
        metadata.response_code = ResponseCode::NoError;
        metadata.recursion_available = true;
        response.metadata = metadata;

        for q in &query.queries {
            response.add_query(q.clone());

            let name = Name::from_str(&q.name().to_ascii())?;
            match lan_ip {
                IpAddr::V4(ipv4) => {
                    if q.query_type() == RecordType::A || q.query_type() == RecordType::ANY {
                        let record = Record::from_rdata(name.clone(), 60, RData::A(A(ipv4)));
                        response.add_answer(record);
                    }
                }
                IpAddr::V6(ipv6) => {
                    if q.query_type() == RecordType::AAAA || q.query_type() == RecordType::ANY {
                        let record = Record::from_rdata(name.clone(), 60, RData::AAAA(AAAA(ipv6)));
                        response.add_answer(record);
                    }
                }
            }
        }

        Ok(response.to_vec()?)
    }

    async fn forward_to_upstream(
        query: &[u8],
        upstream: &[SocketAddr],
    ) -> Result<Vec<u8>, anyhow::Error> {
        let socket = UdpSocket::bind("0.0.0.0:0").await?;

        for addr in upstream {
            socket.send_to(query, addr).await?;

            let mut buf = vec![0u8; 4096];
            match tokio::time::timeout(
                std::time::Duration::from_secs(3),
                socket.recv_from(&mut buf),
            )
            .await
            {
                Ok(Ok((len, _))) => return Ok(buf[..len].to_vec()),
                Ok(Err(e)) => {
                    tracing::warn!("DNS upstream {} recv error: {}", addr, e);
                    continue;
                }
                Err(_) => {
                    tracing::warn!("DNS upstream {} timeout", addr);
                    continue;
                }
            }
        }

        anyhow::bail!("All upstream DNS servers failed")
    }
}

impl StreamTrait for DnsService {
    async fn start(&mut self) -> Result<(), anyhow::Error> {
        if self.abort_handle.is_some() {
            return Err(anyhow::anyhow!("DNS service already running"));
        }

        if !self.config.enabled {
            tracing::info!("Bedrock DNS service disabled, skipping");
            return Ok(());
        }

        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        self.shutdown_tx = Some(shutdown_tx);

        let handle = self.start_server_loop(shutdown_rx).await?;
        self.abort_handle = Some(handle);

        Ok(())
    }

    async fn stop(&mut self) -> Result<(), anyhow::Error> {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(true);
        }

        if let Some(task) = &self.abort_handle {
            task.abort();
        }

        self.abort_handle = None;
        tracing::info!("Bedrock DNS service stopped");
        Ok(())
    }

    fn is_stopped(&self) -> bool {
        self.abort_handle.is_none()
    }

    async fn metadata(&mut self, _key: String, _value: String) -> Result<(), anyhow::Error> {
        Ok(())
    }
}
