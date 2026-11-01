use std::io::{Read, Write};
use std::net::{Ipv4Addr, TcpListener};

/// A plain HTTP server on loopback. The handler receives the request path and returns the
/// response head, ending in a blank line, and the body.
pub struct LocalHttpServer;

impl LocalHttpServer {
    /// Serves until the process ends and returns the port.
    pub fn start<H>(handler: H) -> u16
    where
        H: Fn(&str) -> (String, Vec<u8>) + Send + 'static,
    {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let port = listener.local_addr().unwrap().port();

        std::thread::spawn(move || {
            for mut tcp in listener.incoming().flatten() {
                let mut request = [0u8; 4096];
                let read = tcp.read(&mut request).unwrap_or(0);
                let request = String::from_utf8_lossy(&request[..read]).to_string();
                let path = request
                    .split_whitespace()
                    .nth(1)
                    .unwrap_or("/")
                    .to_string();

                let (head, body) = handler(&path);
                // The client may close early on a refused body, so write errors are expected.
                let _ = tcp.write_all(head.as_bytes());
                let _ = tcp.write_all(&body);
                let _ = tcp.flush();
            }
        });

        port
    }
}
