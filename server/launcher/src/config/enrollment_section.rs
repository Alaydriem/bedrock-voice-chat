use serde::Deserialize;

/// The `server.enrollment` block. The launcher reads only whether a token is set.
#[derive(Debug, Default, Deserialize)]
pub struct EnrollmentSection {
    #[serde(default)]
    pub token: Option<String>,
}
