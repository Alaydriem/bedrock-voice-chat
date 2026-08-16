use common::s2n_quic::provider::event::{ConnectionInfo, ConnectionMeta, Subscriber, events};

use super::PeerIdentityContext;
use crate::stream::quic::CertificateCommonName;

// Captures the mTLS-verified peer certificate Common Name as the TLS handshake
// completes. The rustls provider surfaces the verified chain only through the
// `TlsExporterReady` event, and `Connection::take_tls_context` is never populated by
// it, so this subscriber is the only way to reach the client's identity.
//
// The event fires inside the same handshake-completion step that makes a connection
// eligible for `accept()`, so the CN is always present by the time the accept loop
// reads it.
#[derive(Default)]
pub struct PeerIdentityCapture;

impl Subscriber for PeerIdentityCapture {
    type ConnectionContext = PeerIdentityContext;

    fn create_connection_context(
        &mut self,
        _meta: &ConnectionMeta,
        _info: &ConnectionInfo,
    ) -> Self::ConnectionContext {
        PeerIdentityContext::default()
    }

    fn on_tls_exporter_ready(
        &mut self,
        context: &mut Self::ConnectionContext,
        _meta: &ConnectionMeta,
        event: &events::TlsExporterReady,
    ) {
        let Ok(chain) = event.session.peer_cert_chain_der() else {
            return;
        };
        let Some(leaf) = chain.first() else {
            return;
        };
        if let Some(cn) = CertificateCommonName::from_der(leaf) {
            context.set_cn(cn);
        }
    }
}
