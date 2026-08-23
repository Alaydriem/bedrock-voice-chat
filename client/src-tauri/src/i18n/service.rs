use common::structs::i18n::{LanguagePack, LocaleNegotiator};
use log::warn;
use std::path::PathBuf;

/// Reads compiled language packs from the bundled resource directory.
///
/// English is absent by design: message ids are the English source strings, so a resolved
/// locale of `None` is a complete answer rather than a failure.
pub struct LocalizationService {
    directory: PathBuf,
}

impl LocalizationService {
    pub fn new(directory: PathBuf) -> Self {
        Self { directory }
    }

    pub fn available(&self) -> Vec<String> {
        let Ok(entries) = std::fs::read_dir(&self.directory) else {
            return Vec::new();
        };

        let mut locales: Vec<String> = entries
            .filter_map(Result::ok)
            .filter_map(|entry| {
                let path = entry.path();
                if path.extension()?.to_str()? != "json" {
                    return None;
                }
                Some(path.file_stem()?.to_str()?.to_string())
            })
            .collect();

        locales.sort();
        locales
    }

    pub fn resolve(&self, requested: &str) -> Option<String> {
        LocaleNegotiator::resolve(requested, &self.available())
    }

    pub fn load(&self, locale: &str) -> anyhow::Result<Option<LanguagePack>> {
        let path = self.directory.join(format!("{locale}.json"));
        if !path.exists() {
            return Ok(None);
        }

        let bytes = std::fs::read(&path)?;
        match serde_json::from_slice::<LanguagePack>(&bytes) {
            Ok(pack) => Ok(Some(pack)),
            Err(e) => {
                warn!("language pack {locale} is malformed, falling back to English: {e}");
                Ok(None)
            }
        }
    }
}
