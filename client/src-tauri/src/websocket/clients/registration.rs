use std::sync::Arc;

use super::WebSocketClients;

// Releases a registration however its connection ends: error, shutdown, or dropped peer.
pub struct ClientRegistration {
    clients: Arc<WebSocketClients>,
    id: u64,
}

impl ClientRegistration {
    pub fn new(clients: Arc<WebSocketClients>, name: &str, route: &str) -> Self {
        let id = clients.register(name, route);
        Self { clients, id }
    }

    pub fn count_command(&self) {
        self.clients.count_command(self.id);
    }
}

impl Drop for ClientRegistration {
    fn drop(&mut self) {
        self.clients.release(self.id);
    }
}
