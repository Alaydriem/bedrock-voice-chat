use iroh::PublicKey;
use iroh::endpoint::Connection;

// One held enrollment session.
//
// Held for the life of the server process rather than dialled per exchange. The
// relay cannot dial in: the endpoint is built on iroh's `Minimal` preset, which has
// no discovery, so a bare node id resolves to nothing. Holding the connection the
// server opened is what lets the relay push a challenge to a node behind CGNAT with
// the same code that reaches one with a public address.
//
// `Clone` because an iroh `Connection` is a cheap handle: the session table owns one
// and the serving task another, and both refer to the same connection.
#[derive(Clone)]
pub struct EnrollSession {
    conn: Connection,
    node: PublicKey,
}

impl EnrollSession {
    pub fn new(conn: Connection, node: PublicKey) -> Self {
        Self { conn, node }
    }

    pub fn node(&self) -> PublicKey {
        self.node
    }

    pub fn connection(&self) -> &Connection {
        &self.conn
    }
}
