pub mod code_login_request;

pub use code_login_request::CodeLoginRequest;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateChannelRequest {
    pub name: String,
}
