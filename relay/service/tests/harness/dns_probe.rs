use std::net::SocketAddr;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use hickory_proto::op::{Message, OpCode, Query, ResponseCode};
use hickory_proto::rr::{Name, RData, Record, RecordType, TSigner};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpStream, UdpSocket};

// A DNS client for tests: raw queries over UDP and TCP, and a zone-transfer reader that
// checks the TSIG chain of every message.
pub struct DnsProbe;

impl DnsProbe {
    const TIMEOUT: Duration = Duration::from_secs(5);

    // A query for `name` exactly as written, case and trailing dot included.
    pub fn query(name: &str, kind: RecordType) -> Message {
        let mut message = Message::query();
        message.add_query(Query::query(
            Name::from_ascii(name).expect("a DNS name"),
            kind,
        ));
        message
    }

    pub async fn udp(server: SocketAddr, request: &[u8]) -> Vec<u8> {
        let socket = UdpSocket::bind("127.0.0.1:0").await.expect("binds");
        socket.send_to(request, server).await.expect("sends");

        let mut buffer = vec![0u8; 4096];
        let (length, _) = tokio::time::timeout(Self::TIMEOUT, socket.recv_from(&mut buffer))
            .await
            .expect("the server answers")
            .expect("receives");
        buffer.truncate(length);
        buffer
    }

    pub async fn tcp(server: SocketAddr, request: &[u8]) -> Vec<u8> {
        let mut stream = TcpStream::connect(server).await.expect("connects");
        Self::write_framed(&mut stream, request).await;
        Self::read_framed(&mut stream).await.expect("a reply")
    }

    pub async fn ask(server: SocketAddr, name: &str, kind: RecordType) -> Message {
        let request = Self::query(name, kind).to_vec().expect("encodes");
        Message::from_vec(&Self::udp(server, &request).await).expect("decodes")
    }

    // Pulls a zone over TCP. Signed with `signer` when given; with `verify` set, every
    // response message must carry a valid chained signature. Returns the messages in
    // order. Stops at the second SOA, at a non-NOERROR reply, or when the stream ends.
    pub async fn transfer(
        server: SocketAddr,
        kind: RecordType,
        signer: Option<&TSigner>,
        verify: bool,
    ) -> Vec<Message> {
        let mut request = Self::query("play.bedrockvc.stream.", kind);
        let mut verifier = None;
        if let Some(signer) = signer {
            verifier = request.finalize(signer, Self::now()).expect("signs");
        }
        let bytes = request.to_vec().expect("encodes");

        let mut stream = TcpStream::connect(server).await.expect("connects");
        Self::write_framed(&mut stream, &bytes).await;

        let mut messages = Vec::new();
        let mut soa_count = 0;
        while let Some(raw) = Self::read_framed(&mut stream).await {
            if verify {
                verifier
                    .as_mut()
                    .expect("a signed request")
                    .verify(&raw)
                    .expect("the response message carries a valid chained signature");
            }

            let message = Message::from_vec(&raw).expect("decodes");
            let done = message.metadata.response_code != ResponseCode::NoError;
            soa_count += message
                .answers
                .iter()
                .filter(|r| matches!(r.data, RData::SOA(_)))
                .count();
            messages.push(message);

            if done || soa_count >= 2 {
                break;
            }
        }

        messages
    }

    pub fn records(messages: &[Message]) -> Vec<Record> {
        messages.iter().flat_map(|m| m.answers.clone()).collect()
    }

    pub fn is_notify(message: &Message) -> bool {
        message.metadata.op_code == OpCode::Notify
    }

    pub fn now() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or_default()
    }

    async fn write_framed(stream: &mut TcpStream, bytes: &[u8]) {
        stream
            .write_all(&(bytes.len() as u16).to_be_bytes())
            .await
            .expect("writes the length");
        stream.write_all(bytes).await.expect("writes the message");
    }

    async fn read_framed(stream: &mut TcpStream) -> Option<Vec<u8>> {
        let mut length = [0u8; 2];
        tokio::time::timeout(Self::TIMEOUT, stream.read_exact(&mut length))
            .await
            .ok()?
            .ok()?;

        let mut message = vec![0u8; u16::from_be_bytes(length) as usize];
        tokio::time::timeout(Self::TIMEOUT, stream.read_exact(&mut message))
            .await
            .ok()?
            .ok()?;
        Some(message)
    }
}
