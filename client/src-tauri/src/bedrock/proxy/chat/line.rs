use serde::Serialize;

/// One decoded line of realm chat, on its way to the webview.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ChatLine {
    /// Absent for a server-authored line.
    pub author: Option<String>,
    pub text: String,
    pub system: bool,
}

impl ChatLine {
    pub fn player(author: String, text: String) -> Self {
        Self {
            author: Some(author),
            text,
            system: false,
        }
    }

    pub fn system(text: String) -> Self {
        Self {
            author: None,
            text,
            system: true,
        }
    }
}
