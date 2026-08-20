//! Authentication providers for BVC
//!
//! This module provides authentication for different game platforms:
//! - **Minecraft**: Xbox Live OAuth (authorization code flow)
//!
//! # Example: Minecraft Authentication
//! ```ignore
//! use common::auth::{MinecraftAuthProvider, AuthResult};
//!
//! let provider = MinecraftAuthProvider::new(client_id);
//! let result = provider.authenticate(code, redirect_uri).await?;
//! println!("Welcome, {}!", result.gamertag);
//! ```

mod minecraft;
mod provider;

pub use minecraft::MinecraftAuthProvider;
pub use provider::{AuthError, AuthResult};
