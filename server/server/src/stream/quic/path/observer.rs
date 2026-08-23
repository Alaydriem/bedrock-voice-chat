use common::curia;
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
            curia::warn!("Connection is at the QUIC path limit; datagrams from any further source address will be dropped", { "paths": count, "remote": format!("{:?}", event.new.remote_addr), "path_id": event.new.id });
            return;
        }

        curia::info!("New path on connection", { "paths": count, "remote": format!("{:?}", event.new.remote_addr), "path_id": event.new.id });
    }

    fn on_active_path_updated(
        &mut self,
        _context: &mut Self::ConnectionContext,
        _meta: &ConnectionMeta,
        event: &events::ActivePathUpdated,
    ) {
        curia::info!("Active path changed", { "previous": format!("{:?}", event.previous.remote_addr), "active": format!("{:?}", event.active.remote_addr) });
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
            curia::warn!("Datagram dropped: the connection's path budget is exhausted", { "remote": format!("{:?}", event.remote_addr), "len": event.len });
        }
    }

    fn on_endpoint_datagram_dropped(
        &mut self,
        _meta: &events::EndpointMeta,
        event: &events::EndpointDatagramDropped,
    ) {
        curia::debug!("Datagram dropped before it reached a connection", { "len": event.len, "reason": format!("{:?}", event.reason) });
    }

    fn on_connection_migration_denied(
        &mut self,
        _context: &mut Self::ConnectionContext,
        _meta: &ConnectionMeta,
        event: &events::ConnectionMigrationDenied,
    ) {
        curia::warn!("Connection migration denied; a client that rebinds cannot recover", { "reason": format!("{:?}", event.reason) });
    }
}
