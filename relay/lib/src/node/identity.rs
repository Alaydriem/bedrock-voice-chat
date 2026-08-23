use std::path::{Path, PathBuf};

use iroh::{EndpointAddr, PublicKey, SecretKey};

use super::identity_error::NodeIdentityError;
use super::ticket::PeerTicket;
use super::ticket_error::PeerTicketError;

// A node's identity, persisted to disk.
//
// The key is durable state, not a session value: every `peer` block on the other
// side names this public key, so regenerating it silently would revoke this node
// everywhere at once. A malformed file is therefore an error rather than a cue to
// make a fresh one.
pub struct NodeIdentity {
    secret: SecretKey,
}

impl NodeIdentity {
    // Raw 32 bytes rather than an armoured format: the file is read only by this
    // code, and a length check is a complete validation.
    const FILE_NAME: &'static str = "node.key";
    const KEY_LEN: usize = 32;

    pub fn load_or_create(dir: &str) -> Result<Self, NodeIdentityError> {
        let path = Self::path(dir);

        if path.exists() {
            return Self::load(&path);
        }

        Self::create(&path)
    }

    // The key held somewhere other than this directory — the BVC server keeps it in its
    // database and writes no file. Infallible: 32 bytes is the whole of a secret key, so
    // there is nothing here that can be malformed.
    pub fn from_secret_bytes(bytes: &[u8; Self::KEY_LEN]) -> Self {
        Self {
            secret: SecretKey::from_bytes(bytes),
        }
    }

    pub fn secret_bytes(&self) -> [u8; Self::KEY_LEN] {
        self.secret.to_bytes()
    }

    fn load(path: &Path) -> Result<Self, NodeIdentityError> {
        let bytes = std::fs::read(path).map_err(|source| NodeIdentityError::Read {
            path: path.to_path_buf(),
            source,
        })?;

        let len = bytes.len();
        let bytes: [u8; Self::KEY_LEN] =
            bytes
                .as_slice()
                .try_into()
                .map_err(|_| NodeIdentityError::Malformed {
                    path: path.to_path_buf(),
                    len,
                })?;

        Ok(Self {
            secret: SecretKey::from_bytes(&bytes),
        })
    }

    fn create(path: &Path) -> Result<Self, NodeIdentityError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|source| NodeIdentityError::Write {
                path: parent.to_path_buf(),
                source,
            })?;
        }

        let secret = SecretKey::generate();
        std::fs::write(path, secret.to_bytes()).map_err(|source| NodeIdentityError::Write {
            path: path.to_path_buf(),
            source,
        })?;

        Ok(Self { secret })
    }

    // A ticket naming this node and no addresses.
    //
    // That is the whole of what a granting peer needs: it authorizes by key and
    // never dials back, because a node behind NAT is the case this subsystem
    // exists to serve. Addresses here would be noise an operator copies and
    // nothing reads.
    //
    // Distinct from `PeerEndpoint::ticket`, which reports where a *live* endpoint
    // can be reached. This one answers before any endpoint is bound, which is what
    // lets an operator be granted before the session they are being granted for.
    pub fn peerlink(&self) -> Result<String, PeerTicketError> {
        PeerTicket::mint(&EndpointAddr::new(self.node_id()))
    }

    pub fn node_id(&self) -> PublicKey {
        self.secret.public()
    }

    pub fn secret_key(&self) -> &SecretKey {
        &self.secret
    }

    pub fn path(dir: &str) -> PathBuf {
        Path::new(dir).join(Self::FILE_NAME)
    }
}
