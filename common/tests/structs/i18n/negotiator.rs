use common::structs::i18n::LocaleNegotiator;

fn available() -> Vec<String> {
    vec!["de".into(), "pt_BR".into(), "ru".into(), "zh_CN".into()]
}

#[test]
fn exact_region_match_wins_over_bare_language() {
    let got = LocaleNegotiator::resolve("pt_BR", &available());
    assert_eq!(got, Some("pt_BR".to_string()));
}

#[test]
fn hyphenated_request_matches_underscored_catalog() {
    let got = LocaleNegotiator::resolve("pt-BR", &available());
    assert_eq!(got, Some("pt_BR".to_string()));
}

#[test]
fn region_request_falls_back_to_bare_language() {
    let got = LocaleNegotiator::resolve("ru-RU", &available());
    assert_eq!(got, Some("ru".to_string()));
}

#[test]
fn bare_request_does_not_match_a_region_only_catalog_entry() {
    let got = LocaleNegotiator::resolve("pt", &available());
    assert_eq!(got, None);
}

#[test]
fn unknown_locale_resolves_to_nothing_so_the_caller_uses_english() {
    let got = LocaleNegotiator::resolve("is-IS", &available());
    assert_eq!(got, None);
}

#[test]
fn matching_is_case_insensitive_because_os_locales_vary() {
    let got = LocaleNegotiator::resolve("ZH-cn", &available());
    assert_eq!(got, Some("zh_CN".to_string()));
}
