pub struct LocaleNegotiator;

impl LocaleNegotiator {
    // Resolves an OS locale against the locales actually shipped. `None` means the caller
    // should use English, which needs no pack because the message ids are English.
    pub fn resolve(requested: &str, available: &[String]) -> Option<String> {
        let normalized = Self::normalize(requested);

        if let Some(exact) = available
            .iter()
            .find(|candidate| Self::normalize(candidate) == normalized)
        {
            return Some(exact.clone());
        }

        let language = normalized.split('_').next()?.to_string();
        available
            .iter()
            .find(|candidate| Self::normalize(candidate) == language)
            .cloned()
    }

    fn normalize(locale: &str) -> String {
        locale.replace('-', "_").to_lowercase()
    }
}
