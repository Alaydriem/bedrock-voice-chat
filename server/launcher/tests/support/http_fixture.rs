use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;

use super::{HttpReply, RecordedRequest};

/// A local HTTP server with fixed replies per path. Unknown paths get 404. Every request is
/// recorded, body included.
pub struct HttpFixture {
    port: u16,
    routes: Arc<Mutex<HashMap<String, HttpReply>>>,
    requests: Arc<Mutex<Vec<RecordedRequest>>>,
}

impl HttpFixture {
    pub fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let routes: Arc<Mutex<HashMap<String, HttpReply>>> = Arc::default();
        let requests: Arc<Mutex<Vec<RecordedRequest>>> = Arc::default();
        let served = routes.clone();
        let recorded = requests.clone();
        thread::spawn(move || {
            let mut stalled = Vec::new();
            for stream in listener.incoming().flatten() {
                let Some(request) = Self::read_request(&stream) else {
                    continue;
                };
                let reply = served.lock().unwrap().get(&request.path).cloned();
                recorded.lock().unwrap().push(request);
                match reply {
                    Some(HttpReply::Body(body)) => Self::respond(stream, 200, &body),
                    Some(HttpReply::Status(code)) => Self::respond(stream, code, &[]),
                    Some(HttpReply::Stall) => stalled.push(stream),
                    None => Self::respond(stream, 404, &[]),
                }
            }
        });
        Self {
            port,
            routes,
            requests,
        }
    }

    pub fn route(&self, path: &str, reply: HttpReply) {
        self.routes.lock().unwrap().insert(path.to_string(), reply);
    }

    pub fn url(&self, path: &str) -> String {
        format!("http://127.0.0.1:{}{path}", self.port)
    }

    pub fn requests(&self) -> Vec<RecordedRequest> {
        self.requests.lock().unwrap().clone()
    }

    fn read_request(stream: &TcpStream) -> Option<RecordedRequest> {
        let mut reader = BufReader::new(stream.try_clone().ok()?);
        let mut request_line = String::new();
        reader.read_line(&mut request_line).ok()?;
        let mut parts = request_line.split_whitespace();
        let method = parts.next()?.to_string();
        let path = parts.next()?.to_string();

        let mut headers = Vec::new();
        loop {
            let mut header = String::new();
            if reader.read_line(&mut header).unwrap_or(0) <= 2 {
                break;
            }
            if let Some((name, value)) = header.trim_end().split_once(':') {
                headers.push((name.trim().to_string(), value.trim().to_string()));
            }
        }

        let length = headers
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
            .and_then(|(_, value)| value.parse::<usize>().ok())
            .unwrap_or(0);
        let mut body = vec![0u8; length];
        reader.read_exact(&mut body).ok()?;

        Some(RecordedRequest {
            method,
            path,
            headers,
            body,
        })
    }

    fn respond(mut stream: TcpStream, code: u16, body: &[u8]) {
        let head = format!(
            "HTTP/1.1 {code} Fixture\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        let _ = stream.write_all(head.as_bytes());
        let _ = stream.write_all(body);
    }
}
