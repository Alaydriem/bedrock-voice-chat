use common::s2n_quic::provider::event::{ConnectionInfo, ConnectionMeta, Subscriber, events};

use super::PathObserverContext;

// Records how many network paths a connection accumulates and why datagrams are
// discarded. A client behind a carrier translator whose source address rotates
// shows up here as a rising path count followed by PathLimitExceeded drops; without
// this the same failure is indistinguishable from ordinary packet loss.
#[derive(Default)]
pub struct PathObserver;

impl Subscriber for PathObserver {
    type ConnectionContext = PathObserverContext;

    fn create_connection_context(
        &mut self,
        _meta: &ConnectionMeta,
        _info: &ConnectionInfo,
    ) -> Self::ConnectionContext {
        PathObserverContext::new()
    }

    fn on_path_created(
        &mut self,
        context: &mut Self::ConnectionContext,
        _meta: &ConnectionMeta,
        event: &events::PathCreated,
    ) {
        let count = context.record_path();

        if PathObserverContext::is_near_limit(count) {
            tracing::warn!(
                paths = count,
                remote = ?event.new.remote_addr,
                path_id = event.new.id,
                "Connection is at the QUIC path limit; datagrams from any further source address will be dropped"
            );
            return;
        }

        tracing::info!(
            paths = count,
            remote = ?event.new.remote_addr,
            path_id = event.new.id,
            "New path on connection"
        );
    }

    fn on_active_path_updated(
        &mut self,
        _context: &mut Self::ConnectionContext,
        _meta: &ConnectionMeta,
        event: &events::ActivePathUpdated,
    ) {
        tracing::info!(
            previous = ?event.previous.remote_addr,
            active = ?event.active.remote_addr,
            "Active path changed"
        );
    }

    fn on_datagram_dropped(
        &mut self,
        _context: &mut Self::ConnectionContext,
        _meta: &ConnectionMeta,
        event: &events::DatagramDropped,
    ) {
        if matches!(
            event.reason,
            events::DatagramDropReason::PathLimitExceeded { .. }
        ) {
            tracing::warn!(
                remote = ?event.remote_addr,
                len = event.len,
                "Datagram dropped: the connection's path budget is exhausted"
            );
        }
    }

    fn on_endpoint_datagram_dropped(
        &mut self,
        _meta: &events::EndpointMeta,
        event: &events::EndpointDatagramDropped,
    ) {
        tracing::debug!(
            len = event.len,
            reason = ?event.reason,
            "Datagram dropped before it reached a connection"
        );
    }

    fn on_connection_migration_denied(
        &mut self,
        _context: &mut Self::ConnectionContext,
        _meta: &ConnectionMeta,
        event: &events::ConnectionMigrationDenied,
    ) {
        tracing::warn!(
            reason = ?event.reason,
            "Connection migration denied; a client that rebinds cannot recover"
        );
    }
}
