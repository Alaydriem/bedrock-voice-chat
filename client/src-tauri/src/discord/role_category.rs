use strum::{Display, EnumIter, IntoEnumIterator};

// Variant names are the Flagsmith slugs (kebab-cased), sent as
// `discord-role-<variant>` (e.g. GiftedBvcAccess -> `discord-role-gifted-bvc-access`).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Display, EnumIter)]
#[strum(serialize_all = "kebab-case")]
pub enum RoleCategory {
    FoundationalSupporter,
    Supporter,
    Sponsor,
    GiftedBvcAccess,
}

impl RoleCategory {
    // Emitted alongside any matched category so a single segment can gate
    // "is an entitled member at all", independent of tier.
    const UMBRELLA: &'static str = "supporter-any";

    // Discord role IDs (Patreon- and YouTube-synced) that collapse to this tier.
    pub fn role_ids(self) -> &'static [&'static str] {
        match self {
            Self::FoundationalSupporter => &["1519548906642346054", "1519344428525813962"],
            Self::Supporter => &["1447055413064368239", "1447080496214315131"],
            Self::Sponsor => &["1447055535294906440", "1447080496214315132"],
            Self::GiftedBvcAccess => &["1519551186934562917", "1462185945557368964"],
        }
    }

    pub fn matches(self, role_ids: &[String]) -> bool {
        self.role_ids()
            .iter()
            .any(|id| role_ids.iter().any(|r| r == id))
    }

    // Category slugs the member qualifies for (holds any role in a category),
    // plus the umbrella when at least one category matches. Slugs omit the
    // `discord-role-` prefix.
    pub fn labels_for(role_ids: &[String]) -> Vec<String> {
        let mut labels: Vec<String> = Self::iter()
            .filter(|c| c.matches(role_ids))
            .map(|c| c.to_string())
            .collect();
        if !labels.is_empty() {
            labels.push(Self::UMBRELLA.to_string());
        }
        labels
    }
}
