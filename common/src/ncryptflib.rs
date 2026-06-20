// Core ncryptf exports (always available without rocket)
pub use ncryptf::{
    Authorization, Keypair, NcryptfError, Request, Response, Signature, Token, client,
    randombytes_buf, shared, shared::ExportableEncryptionKeyData,
};

// Server-only: auth, ek_route macro and rocket module require rocket feature
#[cfg(feature = "server")]
pub use ncryptf::auth;

#[cfg(feature = "server")]
pub use ncryptf::ek_route;

#[cfg(feature = "server")]
pub use ncryptf::rocket;
