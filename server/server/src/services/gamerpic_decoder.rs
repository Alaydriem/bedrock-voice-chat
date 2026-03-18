use base64::{engine::general_purpose, Engine as _};

pub struct GamerpicDecoder;

impl GamerpicDecoder {
    pub fn decode(raw: Option<String>) -> Option<String> {
        raw.map(|value| {
            match general_purpose::STANDARD.decode(&value) {
                Ok(bytes) => match String::from_utf8(bytes) {
                    Ok(decoded) => Self::extract_url(&decoded).unwrap_or(value),
                    _ => value,
                },
                Err(_) => value,
            }
        })
    }

    fn extract_url(decoded: &str) -> Option<String> {
        if let Some(pos) = decoded.find("http") {
            let url = &decoded[pos..];
            // Trim any trailing non-URL characters
            let url = url.trim_end_matches('|');
            Some(url.to_string())
        } else {
            None
        }
    }
}
