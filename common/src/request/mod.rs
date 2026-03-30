pub mod audio;
pub mod code_login_request;
pub mod link_java_identity_request;
pub mod login;

pub use audio::{AudioFileListQuery, AudioPlayRequest, AudioStopRequest, GameAudioContext};
pub use code_login_request::CodeLoginRequest;
pub use link_java_identity_request::LinkJavaIdentityRequest;
pub use login::LoginRequest;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateChannelRequest {
    pub name: String,
}
