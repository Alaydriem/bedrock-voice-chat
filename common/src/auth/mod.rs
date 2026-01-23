//! Authentication providers for BVC
//!
//! This module provides authentication for different game platforms:
//! - **Minecraft**: Xbox Live OAuth (authorization code flow)
//! - **Hytale**: OAuth2 Device Code Flow (RFC 8628)
//!
//! # Example: Minecraft Authentication
//! ```ignore
//! use common::auth::{MinecraftAuthProvider, AuthResult};
//!
//! let provider = MinecraftAuthProvider::new(client_id);
//! let result = provider.authenticate(code, redirect_uri).await?;
//! println!("Welcome, {}!", result.gamertag);
//! ```
//!
//! # Example: Hytale Authentication
//! ```ignore
//! use common::auth::{HytaleAuthProvider, DeviceFlow, PollResult};
//!
//! let provider = HytaleAuthProvider::new();
//! let flow = provider.start_device_flow().await?;
//!
//! println!("Go to {} and enter code: {}", flow.verification_uri, flow.user_code);
//!
//! loop {
//!     match provider.poll(&flow).await? {
//!         PollResult::Success(result) => {
//!             println!("Welcome, {}!", result.gamertag);
//!             break;
//!         }
//!         PollResult::Pending => tokio::time::sleep(Duration::from_secs(flow.interval)).await,
//!         PollResult::Expired => panic!("Code expired"),
//!         PollResult::Denied => panic!("User denied"),
//!         PollResult::SlowDown => tokio::time::sleep(Duration::from_secs(flow.interval + 5)).await,
//!     }
//! }
//! ```

mod hytale;
mod minecraft;
mod provider;

pub use provider::{AuthError, AuthResult};
pub use minecraft::MinecraftAuthProvider;
pub use hytale::{DeviceFlow, HytaleAuthProvider, PollResult};
