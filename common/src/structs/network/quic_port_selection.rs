pub struct QuicPortSelection;

impl QuicPortSelection {
    pub const DEFAULT_PORT: u16 = 443;

    // Ordered, de-duplicated candidate ports for a QUIC handshake. The advertised
    // list is authoritative and its order is the operator's stated preference. The
    // cached value is consulted only after everything the server currently
    // reports, so a port that changed server-side cannot be pinned by a client
    // that authenticated before the change.
    pub fn resolve(advertised: &[u32], scalar: u32, cached: Option<&str>) -> Vec<u16> {
        let mut out: Vec<u16> = Vec::new();

        for port in advertised {
            Self::push(*port, &mut out);
        }

        Self::push(scalar, &mut out);

        if let Some(cached) = cached {
            if let Ok(port) = cached.trim().parse::<u32>() {
                Self::push(port, &mut out);
            }
        }

        if out.is_empty() {
            out.push(Self::DEFAULT_PORT);
        }

        out
    }

    // A zero port is how a server that predates port advertisement reports "not
    // known"; it is not a dialable destination. An out-of-range value is a
    // malformed config entry. Both are dropped so one bad entry cannot strand a
    // client that has other viable candidates.
    fn push(port: u32, out: &mut Vec<u16>) {
        if port == 0 {
            return;
        }

        if let Ok(port) = u16::try_from(port) {
            if !out.contains(&port) {
                out.push(port);
            }
        }
    }
}
