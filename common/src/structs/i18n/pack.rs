use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use ts_rs::TS;

// The separator gettext uses between context and message id inside compiled catalogs.
// Both runtimes build the same key, so it is part of the pack's contract rather than an
// implementation detail of either side.
pub const CONTEXT_SEPARATOR: char = '\u{4}';

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(untagged)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub enum PackEntry {
    One(String),
    Many(Vec<String>),
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct LanguagePack {
    pub v: u32,
    pub locale: String,
    // CLDR plural category names in msgstr index order, resolved when the pack was
    // compiled. Runtimes index into this rather than evaluating a Plural-Forms expression.
    pub plural: Vec<String>,
    pub m: HashMap<String, PackEntry>,
}

impl LanguagePack {
    pub fn lookup(&self, msgid: &str) -> Option<&str> {
        match self.m.get(msgid)? {
            PackEntry::One(text) => Some(text.as_str()),
            PackEntry::Many(forms) => forms.first().map(String::as_str),
        }
    }

    pub fn lookup_context(&self, ctx: &str, msgid: &str) -> Option<&str> {
        self.lookup(&format!("{ctx}{CONTEXT_SEPARATOR}{msgid}"))
    }

    // A category absent from `plural` falls to the last form. Gettext catalogs cover the
    // categories a locale uses for integers; CLDR may name one the catalog has no form
    // for, such as Russian `other`, which applies only to fractions.
    pub fn plural_index(&self, category: &str) -> usize {
        self.plural
            .iter()
            .position(|known| known == category)
            .unwrap_or_else(|| self.plural.len().saturating_sub(1))
    }

    pub fn lookup_plural(&self, msgid: &str, category: &str) -> Option<&str> {
        match self.m.get(msgid)? {
            PackEntry::One(text) => Some(text.as_str()),
            PackEntry::Many(forms) => {
                let index = self.plural_index(category).min(forms.len().saturating_sub(1));
                forms.get(index).map(String::as_str)
            }
        }
    }
}
