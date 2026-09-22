/// What the bytes a connection has sent so far say about it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HeadVerdict {
    /// A WebSocket handshake. The existing accept path owns it.
    Upgrade,
    /// A plain HTTP request, with its target split.
    Http { path: String, query: String },
    /// Not enough has arrived to tell.
    Incomplete,
}

impl HeadVerdict {
    const TERMINATOR: &'static str = "\r\n\r\n";

    pub fn classify(head: &str) -> Self {
        // Decided as soon as the header is present, without waiting for the rest of the head:
        // the handshake reads the whole request itself, so handing it over early costs nothing.
        if Self::names_upgrade(head) {
            return Self::Upgrade;
        }

        // Until the blank line, a header naming the upgrade may still be in bytes that have not
        // arrived. Answering as HTTP here would refuse a WebSocket client its handshake.
        if !head.contains(Self::TERMINATOR) {
            return Self::Incomplete;
        }

        match Self::target_of(head) {
            Some((path, query)) => Self::Http { path, query },
            None => Self::Incomplete,
        }
    }

    fn names_upgrade(head: &str) -> bool {
        head.lines().any(|line| {
            let Some((name, value)) = line.split_once(':') else {
                return false;
            };
            name.trim().eq_ignore_ascii_case("upgrade")
                && value.trim().eq_ignore_ascii_case("websocket")
        })
    }

    fn target_of(head: &str) -> Option<(String, String)> {
        let request_line = head.lines().next()?;
        let mut parts = request_line.split_whitespace();
        let _method = parts.next()?;
        let target = parts.next()?;

        Some(match target.split_once('?') {
            Some((path, query)) => (path.to_string(), query.to_string()),
            None => (target.to_string(), String::new()),
        })
    }
}
