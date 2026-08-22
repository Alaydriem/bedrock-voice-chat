use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerErrorType {
    VersionIncompatible {
        client_version: String,
        server_version: String,
    },
    // Appended, never inserted: postcard encodes the variant index, so a new variant in the
    // middle shifts every later discriminant and mis-decodes packets that were fine before.
    //
    // A client too old to decode this fails to parse the packet and is disconnected anyway,
    // so the failure mode is a worse message rather than a missed revocation.
    CertificateRevoked {
        reason: String,
    },
    // The server already holds as many voice sessions as its operator permits. Unlike the
    // two above, this is not a decision about the client: the same client succeeds once a
    // slot frees, so it must not be treated as terminal.
    AtCapacity {
        limit: u32,
    },
}
