use super::RejectReason;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WebSocketRoute {
    // The request/response command protocol every existing integration speaks. Authentication is
    // per message here, unchanged.
    Command,
    // A push-only diagnostics stream. Authentication has to happen at the upgrade because there
    // is no inbound message to carry a key.
    Metrics,
}

impl WebSocketRoute {
    // Which endpoint a connection is on, for the settings pane that lists them. A
    // metrics subscriber and a command client behave nothing alike, so an operator
    // looking at two rows needs to know which is which.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Command => "command",
            Self::Metrics => "metrics",
        }
    }

    pub fn resolve(uri: &str, configured_key: &str) -> Result<Self, RejectReason> {
        let (path, query) = match uri.split_once('?') {
            Some((path, query)) => (path, Some(query)),
            None => (uri, None),
        };

        // At most one trailing slash is insignificant. Trimming them all would collapse `//` to the
        // empty string, which matters because the empty path is the command protocol.
        let path = match path {
            "" | "/" => "",
            other => other.strip_suffix('/').unwrap_or(other),
        };

        // Only `/metrics` is routed specially. Everything else is the command protocol, because
        // that is what the previous `accept_async` did — it never inspected the path at all, so an
        // integration connecting to `/ws` or using an absolute-form request target upgraded fine.
        // Rejecting those would break third-party clients silently at the handshake, and the
        // command protocol authenticates per message regardless of the path it arrived on.
        if path != "/metrics" {
            return Ok(Self::Command);
        }

        if configured_key.is_empty() {
            return Ok(Self::Metrics);
        }

        match Self::key_from_query(query) {
            None => Err(RejectReason::MissingKey),
            Some(key) if key == configured_key => Ok(Self::Metrics),
            Some(_) => Err(RejectReason::InvalidKey),
        }
    }

    fn key_from_query(query: Option<&str>) -> Option<String> {
        query?.split('&').find_map(|pair| {
            let (name, value) = pair.split_once('=')?;
            if name == "key" {
                Some(Self::percent_decode(value))
            } else {
                None
            }
        })
    }

    // Keys are generated as URL-safe text, so full percent-decoding is unnecessary; `+` and the
    // escapes a browser may still apply to a pasted key are handled so a correct key is never
    // rejected for its spelling.
    fn percent_decode(value: &str) -> String {
        let bytes = value.as_bytes();
        let mut out = Vec::with_capacity(bytes.len());
        let mut i = 0;

        while i < bytes.len() {
            match bytes[i] {
                b'+' => {
                    out.push(b' ');
                    i += 1;
                }
                b'%' if i + 2 < bytes.len() => {
                    let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).ok();
                    match hex.and_then(|h| u8::from_str_radix(h, 16).ok()) {
                        Some(byte) => {
                            out.push(byte);
                            i += 3;
                        }
                        None => {
                            out.push(bytes[i]);
                            i += 1;
                        }
                    }
                }
                other => {
                    out.push(other);
                    i += 1;
                }
            }
        }

        String::from_utf8_lossy(&out).into_owned()
    }
}
