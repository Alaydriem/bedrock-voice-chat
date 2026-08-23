use common::structs::i18n::LanguagePack;
use icu_locale_core::Locale;
use icu_plurals::{PluralCategory, PluralRules};
use std::str::FromStr;

/// Translation for text that lives outside the webview.
///
/// Nothing calls this yet. It exists so the tray menu and OS notifications, when they
/// arrive, read the same pack the frontend reads rather than growing a second catalog
/// that can disagree with the first.
#[allow(dead_code)]
pub struct Localizer {
    pack: Option<LanguagePack>,
    rules: Option<PluralRules>,
}

#[allow(dead_code)]
impl Localizer {
    pub fn new(pack: Option<LanguagePack>) -> Self {
        let rules = pack.as_ref().and_then(|pack| {
            let locale = Locale::from_str(&pack.locale.replace('_', "-")).ok()?;
            PluralRules::try_new_cardinal((&locale).into()).ok()
        });
        Self { pack, rules }
    }

    pub fn t(&self, msgid: &str) -> String {
        self.pack
            .as_ref()
            .and_then(|pack| pack.lookup(msgid))
            .unwrap_or(msgid)
            .to_string()
    }

    pub fn tc(&self, context: &str, msgid: &str) -> String {
        self.pack
            .as_ref()
            .and_then(|pack| pack.lookup_context(context, msgid))
            .unwrap_or(msgid)
            .to_string()
    }

    pub fn tn(&self, singular: &str, plural: &str, n: u64) -> String {
        let Some(pack) = self.pack.as_ref() else {
            return if n == 1 { singular } else { plural }.to_string();
        };

        let Some(rules) = self.rules.as_ref() else {
            return self.t(singular);
        };

        pack.lookup_plural(singular, Self::name(rules.category_for(n)))
            .unwrap_or(if n == 1 { singular } else { plural })
            .to_string()
    }

    // Spelled out rather than derived from Debug, because these names are the contract the
    // pack compiler wrote into `plural` and a formatting change upstream would break the
    // match silently.
    fn name(category: PluralCategory) -> &'static str {
        match category {
            PluralCategory::Zero => "zero",
            PluralCategory::One => "one",
            PluralCategory::Two => "two",
            PluralCategory::Few => "few",
            PluralCategory::Many => "many",
            PluralCategory::Other => "other",
        }
    }
}
