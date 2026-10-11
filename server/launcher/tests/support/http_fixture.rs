use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;

use super::HttpReply;

/// A local HTTP server with fixed replies per path. Unknown paths get 404.
pub struct HttpFixture {
    port: u16,
    routes: Arc<Mutex<HashMap<String, HttpReply>>>,
}

impl HttpFixture {
    pub fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let routes: Arc<Mutex<HashMap<String, HttpReply>>> = Arc::default();
        let served = routes.clone();
        thread::spawn(move || {
            let mut stalled = Vec::new();
            for stream in listener.incoming().flatten() {
                let Some(path) = Self::request_path(&stream) else {
                    continue;
                };
                let reply = served.lock().unwrap().get(&path).cloned();
                match reply {
                    Some(HttpReply::Body(body)) => Self::respond(stream, 200, &body),
                    Some(HttpReply::Status(code)) => Self::respond(stream, code, &[]),
                    Some(HttpReply::Stall) => stalled.push(stream),
                    None => Self::respond(stream, 404, &[]),
                }
            }
        });
        Self { port, routes }
    }

    pub fn route(&self, path: &str, reply: HttpReply) {
        self.routes.lock().unwrap().insert(path.to_string(), reply);
    }

    pub fn url(&self, path: &str) -> String {
        format!("http://127.0.0.1:{}{path}", self.port)
    }

    // The path of the request line; the headers are read and discarded.
    fn request_path(stream: &TcpStream) -> Option<String> {
        let mut reader = BufReader::new(stream.try_clone().ok()?);
        let mut request_line = String::new();
        reader.read_line(&mut request_line).ok()?;
        loop {
            let mut header = String::new();
            if reader.read_line(&mut header).unwrap_or(0) <= 2 {
                break;
            }
        }
        request_line.split_whitespace().nth(1).map(str::to_string)
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
