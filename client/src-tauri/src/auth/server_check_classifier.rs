use common::net::ErrorChain;
use common::structs::reachability::ServerCheckFailure;
use std::error::Error;
use std::io::ErrorKind;

/// Turns a failed `/api/config` request into the reason the sign-in screen gives for it.
pub struct ServerCheckClassifier;

impl ServerCheckClassifier {
    /// Walks the error chain for a typed cause first. Only hyper-util's resolver failure has no
    /// typed form to find, so the rendered chain is the last resort, and only for wording that
    /// hyper-util and reqwest write themselves rather than the OS.
    pub fn classify(error: &(dyn Error + 'static)) -> ServerCheckFailure {
        let mut current = Some(error);

        while let Some(cause) = current {
            if let Some(tls) = cause.downcast_ref::<rustls::Error>() {
                return Self::from_tls(tls);
            }

            if let Some(io) = cause.downcast_ref::<std::io::Error>() {
                match io.kind() {
                    ErrorKind::ConnectionRefused => return ServerCheckFailure::Refused,
                    ErrorKind::TimedOut => return ServerCheckFailure::TimedOut,
                    ErrorKind::ConnectionReset
                    | ErrorKind::ConnectionAborted
                    | ErrorKind::UnexpectedEof => return ServerCheckFailure::ConnectionLost,
                    _ => {}
                }

                // `io::Error::source` skips the error it carries, so a boxed rustls error is
                // only reachable through `get_ref`.
                if let Some(inner) = io.get_ref() {
                    current = Some(inner);
                    continue;
                }
            }

            current = cause.source();
        }

        let rendered = ErrorChain::render(error).to_lowercase();
        if rendered.contains("dns error")
            || rendered.contains("invalid dns name")
            || rendered.contains("invalid international domain name")
        {
            ServerCheckFailure::NameNotFound
        } else if rendered.contains("tls handshake eof") {
            ServerCheckFailure::ConnectionLost
        } else if rendered.contains("timed out") || rendered.contains("deadline has elapsed") {
            ServerCheckFailure::TimedOut
        } else {
            ServerCheckFailure::Other
        }
    }

    pub fn for_status(status: u16) -> ServerCheckFailure {
        if status >= 500 || status == 429 {
            ServerCheckFailure::ServerError { status }
        } else {
            ServerCheckFailure::NotBvc {
                status: Some(status),
            }
        }
    }

    fn from_tls(error: &rustls::Error) -> ServerCheckFailure {
        match error {
            rustls::Error::InvalidCertificate(
                rustls::CertificateError::NotValidForNameContext { presented, .. },
            ) => ServerCheckFailure::WrongCertificateName {
                presented: presented.iter().map(|name| Self::bare_name(name)).collect(),
            },
            _ => ServerCheckFailure::TlsRejected,
        }
    }

    /// rustls reports each name in its debug form, `DnsName("panel.example.com")`.
    fn bare_name(presented: &str) -> String {
        match (presented.find('"'), presented.rfind('"')) {
            (Some(start), Some(end)) if end > start => presented[start + 1..end].to_string(),
            _ => presented.to_string(),
        }
    }
}
