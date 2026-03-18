pub mod code_login_request;

pub use code_login_request::CodeLoginRequest;
pub mod link_java_identity_request;

pub use link_java_identity_request::LinkJavaIdentityRequest;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateChannelRequest {
    pub name: String,
}
