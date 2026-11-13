use std::net::SocketAddr;
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpStream, UdpSocket};

use super::{SrvLookup, SrvWire};

/// Asks the play zone's authoritative nameservers for an SRV record, in order.
pub struct AuthoritativeQuery {
    nameservers: Vec<String>,
    timeout: Duration,
}

impl AuthoritativeQuery {
    pub fn new(nameservers: Vec<String>, timeout: Duration) -> Self {
        Self {
            nameservers,
            timeout,
        }
    }

    // A nameserver that answers settles the question, whatever the answer. A nameserver that
    // does not answer hands the question to the next one.
    pub async fn lookup(&self, host: &str) -> SrvLookup {
        let mut failures = Vec::new();

        for nameserver in &self.nameservers {
            // One deadline covers the name lookup, the UDP query and any TCP retry.
            let attempt = tokio::time::timeout(self.timeout, self.ask(nameserver, host))
                .await
                .unwrap_or_else(|_| Err("no answer within the timeout".to_string()));
            match attempt {
                Ok(SrvLookup::Failed(reason)) | Err(reason) => {
                    failures.push(format!("{nameserver}: {reason}"));
                }
                Ok(settled) => return settled,
            }
        }

        if failures.is_empty() {
            failures.push("no nameservers are configured".to_string());
        }
        SrvLookup::Failed(failures.join("; "))
    }

    async fn ask(&self, nameserver: &str, host: &str) -> Result<SrvLookup, String> {
        let address = self.resolve(nameserver).await?;
        let query = SrvWire::query(host, false)?;
        let id = query.metadata.id;
        let bytes = SrvWire::encode(&query)?;

        let reply = self.over_udp(address, id, &bytes).await?;

        if !reply.metadata.truncation {
            return Ok(SrvWire::classify(&reply));
        }

        let reply = self.over_tcp(address, id, &bytes).await?;
        Ok(SrvWire::classify(&reply))
    }

    async fn resolve(&self, nameserver: &str) -> Result<SocketAddr, String> {
        let mut addresses = tokio::net::lookup_host(nameserver)
            .await
            .map_err(|error| error.to_string())?;
        addresses
            .next()
            .ok_or_else(|| "the nameserver name has no address".to_string())
    }

    async fn over_udp(
        &self,
        address: SocketAddr,
        id: u16,
        bytes: &[u8],
    ) -> Result<hickory_proto::op::Message, String> {
        let local = if address.is_ipv4() {
            "0.0.0.0:0"
        } else {
            "[::]:0"
        };
        let socket = UdpSocket::bind(local)
            .await
            .map_err(|error| error.to_string())?;
        socket
            .connect(address)
            .await
            .map_err(|error| error.to_string())?;
        socket
            .send(bytes)
            .await
            .map_err(|error| error.to_string())?;

        let mut buffer = [0u8; 4096];
        loop {
            let length = socket
                .recv(&mut buffer)
                .await
                .map_err(|error| error.to_string())?;
            // A datagram that does not answer this query is not an answer.
            if let Ok(message) = SrvWire::decode(&buffer[..length])
                && message.metadata.id == id
            {
                return Ok(message);
            }
        }
    }

    async fn over_tcp(
        &self,
        address: SocketAddr,
        id: u16,
        bytes: &[u8],
    ) -> Result<hickory_proto::op::Message, String> {
        let mut stream = TcpStream::connect(address)
            .await
            .map_err(|error| error.to_string())?;

        let mut framed = (bytes.len() as u16).to_be_bytes().to_vec();
        framed.extend_from_slice(bytes);
        stream
            .write_all(&framed)
            .await
            .map_err(|error| error.to_string())?;

        let mut prefix = [0u8; 2];
        stream
            .read_exact(&mut prefix)
            .await
            .map_err(|error| error.to_string())?;
        let mut body = vec![0u8; u16::from_be_bytes(prefix) as usize];
        stream
            .read_exact(&mut body)
            .await
            .map_err(|error| error.to_string())?;

        let message = SrvWire::decode(&body)?;
        if message.metadata.id != id {
            return Err("the TCP reply answers another query".to_string());
        }
        Ok(message)
    }
}
