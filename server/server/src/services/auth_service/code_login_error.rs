use rocket::http::Status;

use crate::services::{AuthCodeError, AuthError};

#[derive(Debug, thiserror::Error)]
pub enum CodeLoginError {
    #[error("code login feature disabled")]
    FeatureDisabled,
    #[error("code error: {0}")]
    Code(#[from] AuthCodeError),
    #[error("auth error: {0}")]
    Auth(#[from] AuthError),
}

impl CodeLoginError {
    pub fn to_status(&self) -> Status {
        match self {
            CodeLoginError::FeatureDisabled => Status::NotFound,
            CodeLoginError::Code(AuthCodeError::CodeNotFound) => Status::NotFound,
            CodeLoginError::Code(AuthCodeError::GamertagMismatch)
            | CodeLoginError::Code(AuthCodeError::CodeAlreadyUsed) => Status::Forbidden,
            CodeLoginError::Code(AuthCodeError::CodeExpired) => Status::Gone,
            CodeLoginError::Code(_) => Status::InternalServerError,
            CodeLoginError::Auth(AuthError::PlayerNotFound)
            | CodeLoginError::Auth(AuthError::PlayerBanished) => Status::Forbidden,
            CodeLoginError::Auth(_) => Status::InternalServerError,
        }
    }
}
